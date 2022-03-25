// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use crate::{prelude::*, split_circuits::*};
use snarkvm_algorithms::prelude::*;
use snarkvm_r1cs::{ConstraintSynthesizer, ConstraintSystem, TestConstraintSystem};
use snarkvm_utilities::ToBytes;

use itertools::Itertools;
use rand::thread_rng;

// TODO (raychu86): Add additional tests for different number of inputs and outputs.

fn dpc_execute_circuits_test<N: Network>(
    expected_input_num_constraints: usize,
    expected_output_num_constraints: usize,
    expected_value_check_num_constraints: usize,
) {
    let rng = &mut thread_rng();

    let recipient = Account::new(rng);
    let amount = AleoAmount::from_gate(10);
    let request: Request<N> = Request::new_coinbase(recipient.address(), amount, false, rng).unwrap();
    let response: Response<N> = ResponseBuilder::new()
        .add_request(request.clone())
        .add_output(Output::new(recipient.address(), amount, None, None).unwrap())
        .build(rng)
        .unwrap();

    //////////////////////////////////////////////////////////////////////////

    // Fetch the ledger root, serial numbers, and program ID.
    let ledger_root = LedgerTree::<N>::new().unwrap().root();
    let serial_numbers = request.to_serial_numbers().unwrap();
    let program_id = request.to_program_id().unwrap();

    // Fetch the commitments and ciphertexts.
    let commitments = response.commitments();

    // Compute the value balance.
    let mut value_balance = AleoAmount::ZERO;
    for record in request.records().iter() {
        value_balance = value_balance.add(record.value());
    }
    for record in response.records().iter() {
        value_balance = value_balance.sub(record.value());
    }

    // Compute the local transitions root.
    let local_transitions_root = Transitions::<N>::new().unwrap().root();

    // Compute the transition ID.
    let transition_id = Transition::<N>::compute_transition_id(&serial_numbers, &commitments).unwrap();

    //////////////////////////////////////////////////////////////////////////

    // Generate input circuit parameters and proof.
    let (input_proving_key, input_verifying_key) =
        <N as Network>::InputSNARK::setup(&InputCircuit::<N>::blank(), &mut SRS::CircuitSpecific(rng)).unwrap();

    // Compute the input circuit proofs.
    let mut input_proofs = Vec::with_capacity(N::MAX_NUM_INPUT_RECORDS);
    let mut input_public_variables = Vec::with_capacity(N::MAX_NUM_INPUT_RECORDS);
    for (
        ((((record, serial_number), ledger_proof), signature), input_value_commitment),
        input_value_commitment_randomness,
    ) in request
        .records()
        .iter()
        .zip_eq(request.to_serial_numbers().unwrap().iter())
        .zip_eq(request.ledger_proofs())
        .zip_eq(request.signatures())
        .zip_eq(response.input_value_commitments())
        .zip_eq(response.input_value_commitment_randomness())
    {
        // Check that the input constraint system was satisfied.
        let mut input_cs = TestConstraintSystem::<N::InnerScalarField>::new();

        let input_public = InputPublicVariables::<N>::new(
            *serial_number,
            input_value_commitment.clone(),
            ledger_root,
            local_transitions_root,
            program_id,
        );
        let input_private = InputPrivateVariables::<N>::new(
            record.clone(),
            ledger_proof.clone(),
            signature.clone(),
            *input_value_commitment_randomness,
        )
        .unwrap();

        let input_circuit = InputCircuit::<N>::new(input_public.clone(), input_private);
        input_circuit.generate_constraints(&mut input_cs.ns(|| "Input circuit")).unwrap();

        let candidate_input_num_constraints = input_cs.num_constraints();
        let (num_non_zero_a, num_non_zero_b, num_non_zero_c) = input_cs.num_non_zero();

        if !input_cs.is_satisfied() {
            println!("=========================================================");
            println!("Input circuit num constraints: {}", candidate_input_num_constraints);
            println!("Unsatisfied constraints:\n{}", input_cs.which_is_unsatisfied().unwrap());
            println!("=========================================================");
        }

        println!("=========================================================");
        println!("Input circuit num constraints: {}", candidate_input_num_constraints);
        assert_eq!(expected_input_num_constraints, candidate_input_num_constraints);
        println!("=========================================================");

        println!("=========================================================");
        println!("Input circuit num non_zero_a: {}", num_non_zero_a);
        println!("Input circuit num non_zero_b: {}", num_non_zero_b);
        println!("Input circuit num non_zero_c: {}", num_non_zero_c);
        println!("=========================================================");

        assert!(input_cs.is_satisfied());

        //////////////////////////////////////////////////////////////////////////

        let input_proof = <N as Network>::InputSNARK::prove(&input_proving_key, &input_circuit, rng).unwrap();
        assert_eq!(N::INPUT_PROOF_SIZE_IN_BYTES, input_proof.to_bytes_le().unwrap().len());

        // Verify that the inner circuit proof passes.
        assert!(<N as Network>::InputSNARK::verify(&input_verifying_key, &input_public, &input_proof).unwrap());

        //////////////////////////////////////////////////////////////////////////

        input_proofs.push(input_proof.into());
        input_public_variables.push(input_public);
    }

    //////////////////////////////////////////////////////////////////////////

    // Generate output circuit parameters and proof.
    let (output_proving_key, output_verifying_key) =
        <N as Network>::OutputSNARK::setup(&OutputCircuit::<N>::blank(), &mut SRS::CircuitSpecific(rng)).unwrap();

    // Compute the output circuit proofs.
    let mut output_proofs = Vec::with_capacity(N::MAX_NUM_OUTPUT_RECORDS);
    let mut output_public_variables = Vec::with_capacity(N::MAX_NUM_OUTPUT_RECORDS);
    for (
        (((record, commitment), encryption_randomness), output_value_commitment),
        output_value_commitment_randomness,
    ) in response
        .records()
        .iter()
        .zip_eq(response.commitments())
        .zip_eq(response.encryption_randomness())
        .zip_eq(response.output_value_commitments())
        .zip_eq(response.output_value_commitment_randomness())
    {
        // Check that the output constraint system was satisfied.
        let mut output_cs = TestConstraintSystem::<N::InnerScalarField>::new();

        let output_public = OutputPublicVariables::<N>::new(commitment, output_value_commitment.clone(), program_id);
        let output_private = OutputPrivateVariables::<N>::new(
            record.clone(),
            *encryption_randomness,
            *output_value_commitment_randomness,
        )
        .unwrap();

        let output_circuit = OutputCircuit::<N>::new(output_public.clone(), output_private);
        output_circuit.generate_constraints(&mut output_cs.ns(|| "Output circuit")).unwrap();

        let candidate_output_num_constraints = output_cs.num_constraints();
        let (num_non_zero_a, num_non_zero_b, num_non_zero_c) = output_cs.num_non_zero();

        if !output_cs.is_satisfied() {
            println!("=========================================================");
            println!("Output circuit num constraints: {}", candidate_output_num_constraints);
            println!("Unsatisfied constraints:\n{}", output_cs.which_is_unsatisfied().unwrap());
            println!("=========================================================");
        }

        println!("=========================================================");
        println!("Output circuit num constraints: {}", candidate_output_num_constraints);
        assert_eq!(expected_output_num_constraints, candidate_output_num_constraints);
        println!("=========================================================");

        println!("=========================================================");
        println!("Output circuit num non_zero_a: {}", num_non_zero_a);
        println!("Output circuit num non_zero_b: {}", num_non_zero_b);
        println!("Output circuit num non_zero_c: {}", num_non_zero_c);
        println!("=========================================================");

        assert!(output_cs.is_satisfied());

        //////////////////////////////////////////////////////////////////////////

        let output_proof = <N as Network>::OutputSNARK::prove(&output_proving_key, &output_circuit, rng).unwrap();
        assert_eq!(N::OUTPUT_PROOF_SIZE_IN_BYTES, output_proof.to_bytes_le().unwrap().len());

        // Verify that the inner circuit proof passes.
        assert!(<N as Network>::OutputSNARK::verify(&output_verifying_key, &output_public, &output_proof).unwrap());

        //////////////////////////////////////////////////////////////////////////

        output_proofs.push(output_proof.into());
        output_public_variables.push(output_public);
    }

    //////////////////////////////////////////////////////////////////////////

    // Generate value check circuit parameters and proof.
    let (value_check_proving_key, value_check_verifying_key) =
        <N as Network>::ValueCheckSNARK::setup(&ValueCheckCircuit::<N>::blank(), &mut SRS::CircuitSpecific(rng))
            .unwrap();

    // Check that the value check constraint system was satisfied.
    let mut value_check_cs = TestConstraintSystem::<N::InnerScalarField>::new();

    let value_check_public_variables =
        ValueCheckPublicVariables::<N>::new(value_balance, response.value_balance_commitment().clone());
    let value_check_private_variables = ValueCheckPrivateVariables::<N>::new(
        Transition::<N>::compute_transition_id(&request.to_serial_numbers().unwrap(), &response.commitments()).unwrap(),
        response.input_value_commitments().clone(),
        response.output_value_commitments().clone(),
    )
    .unwrap();

    let value_check_circuit =
        ValueCheckCircuit::<N>::new(value_check_public_variables.clone(), value_check_private_variables);
    value_check_circuit.generate_constraints(&mut value_check_cs.ns(|| "Value check circuit")).unwrap();

    let candidate_value_check_num_constraints = value_check_cs.num_constraints();
    let (num_non_zero_a, num_non_zero_b, num_non_zero_c) = value_check_cs.num_non_zero();

    if !value_check_cs.is_satisfied() {
        println!("=========================================================");
        println!("Value check circuit num constraints: {}", candidate_value_check_num_constraints);
        println!("Unsatisfied constraints:\n{}", value_check_cs.which_is_unsatisfied().unwrap());
        println!("=========================================================");
    }

    println!("=========================================================");
    println!("Value check circuit num constraints: {}", candidate_value_check_num_constraints);
    assert_eq!(expected_value_check_num_constraints, candidate_value_check_num_constraints);
    println!("=========================================================");

    println!("=========================================================");
    println!("Value check circuit num non_zero_a: {}", num_non_zero_a);
    println!("Value check circuit num non_zero_b: {}", num_non_zero_b);
    println!("Value check circuit num non_zero_c: {}", num_non_zero_c);
    println!("=========================================================");

    assert!(value_check_cs.is_satisfied());

    //////////////////////////////////////////////////////////////////////////

    let value_check_proof =
        <N as Network>::ValueCheckSNARK::prove(&value_check_proving_key, &value_check_circuit, rng).unwrap();
    assert_eq!(N::VALUE_CHECK_PROOF_SIZE_IN_BYTES, value_check_proof.to_bytes_le().unwrap().len());

    // Verify that the inner circuit proof passes.
    assert!(
        <N as Network>::ValueCheckSNARK::verify(
            &value_check_verifying_key,
            &value_check_public_variables,
            &value_check_proof
        )
        .unwrap()
    );

    //////////////////////////////////////////////////////////////////////////

    // Construct the execution.
    let execution = Execution::<N>::from(None, input_proofs, output_proofs, value_check_proof.into()).unwrap();

    // Verify that the program proof passes.
    assert!(execution.verify(
        &input_verifying_key,
        &output_verifying_key,
        &value_check_verifying_key,
        &input_public_variables,
        &output_public_variables,
        &value_check_public_variables,
        transition_id
    ));
}

mod testnet1 {
    use super::*;
    use crate::testnet1::*;

    #[test]
    fn test_dpc_execute_circuits() {
        dpc_execute_circuits_test::<Testnet1>(113181, 18731, 11069);
    }
}

mod testnet2 {
    use super::*;
    use crate::testnet2::*;

    #[test]
    fn test_dpc_execute_circuits() {
        dpc_execute_circuits_test::<Testnet2>(113181, 18731, 11069);
    }
}
