// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::{
    account::{ACCOUNT_COMMITMENT_INPUT, ACCOUNT_ENCRYPTION_INPUT},
    testnet2::{Testnet2Components, DPC},
    InnerCircuitVerifierInput,
    Network,
    OuterCircuitVerifierInput,
    Parameters,
    ProgramLocalData,
    Transaction,
};
use snarkvm_algorithms::{
    commitment::{Blake2sCommitment, PedersenCompressedCommitment},
    crh::BoweHopwoodPedersenCompressedCRH,
    crypto_hash::PoseidonCryptoHash,
    define_merkle_tree_parameters,
    encryption::GroupEncryption,
    prelude::*,
    prf::Blake2s,
    signature::Schnorr,
    snark::groth16::Groth16,
};
use snarkvm_curves::{
    bls12_377::Bls12_377,
    bw6_761::BW6_761,
    edwards_bls12::{EdwardsParameters, EdwardsProjective as EdwardsBls12},
    PairingEngine,
};
use snarkvm_gadgets::{
    algorithms::{
        commitment::{Blake2sCommitmentGadget, PedersenCompressedCommitmentGadget},
        crh::BoweHopwoodPedersenCompressedCRHGadget,
        crypto_hash::PoseidonCryptoHashGadget,
        encryption::GroupEncryptionGadget,
        prf::Blake2sGadget,
        signature::SchnorrGadget,
        snark::Groth16VerifierGadget,
    },
    curves::{bls12_377::PairingGadget, edwards_bls12::EdwardsBls12Gadget},
};
use snarkvm_marlin::{
    constraints::{snark::MarlinSNARK, verifier::MarlinVerificationGadget},
    marlin::MarlinTestnet2Mode,
    FiatShamirAlgebraicSpongeRng,
    PoseidonSponge,
};
use snarkvm_polycommit::marlin_pc::{marlin_kzg10::MarlinKZG10Gadget, MarlinKZG10};

use once_cell::sync::OnceCell;
use snarkvm_polycommit::PolynomialCommitment;

macro_rules! dpc_setup {
    ($fn_name: ident, $static_name: ident, $type_name: ident, $setup_msg: expr) => {
        #[inline]
        fn $fn_name() -> &'static Self::$type_name {
            static $static_name: OnceCell<<Testnet2Parameters as Parameters>::$type_name> = OnceCell::new();
            $static_name.get_or_init(|| Self::$type_name::setup($setup_msg))
        }
    };
}

pub type Testnet2DPC = DPC<Testnet2Parameters>;
pub type Testnet2Transaction = Transaction<Testnet2Parameters>;

define_merkle_tree_parameters!(
    CommitmentMerkleTreeParameters,
    <Testnet2Parameters as Parameters>::RecordCommitmentTreeCRH,
    32
);

pub struct Testnet2Parameters;

// TODO (raychu86): Optimize each of the window sizes in the type declarations below.
#[rustfmt::skip]
impl Parameters for Testnet2Parameters {
    const NETWORK_ID: u8 = Network::Testnet2.id();

    const NUM_INPUT_RECORDS: usize = 2;
    const NUM_OUTPUT_RECORDS: usize = 2;

    type InnerCurve = Bls12_377;
    type OuterCurve = BW6_761;

    type InnerScalarField = <Self::InnerCurve as PairingEngine>::Fr;
    type OuterScalarField = <Self::OuterCurve as PairingEngine>::Fr;
    type OuterBaseField = <Self::OuterCurve as PairingEngine>::Fq;

    type InnerSNARK = Groth16<Self::InnerCurve, InnerCircuitVerifierInput<Testnet2Parameters>>;
    type OuterSNARK = Groth16<Self::OuterCurve, OuterCircuitVerifierInput<Testnet2Parameters>>;

    type AccountCommitmentScheme = PedersenCompressedCommitment<EdwardsBls12, 8, 192>;
    type AccountCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls12, Self::InnerScalarField, EdwardsBls12Gadget, 8, 192>;
    type AccountCommitment = <Self::AccountCommitmentScheme as CommitmentScheme>::Output;

    type AccountEncryptionScheme = GroupEncryption<EdwardsBls12>;
    type AccountEncryptionGadget = GroupEncryptionGadget<EdwardsBls12, Self::InnerScalarField, EdwardsBls12Gadget>;

    type AccountSignatureScheme = Schnorr<EdwardsBls12>;
    type AccountSignatureGadget = SchnorrGadget<EdwardsBls12, Self::InnerScalarField, EdwardsBls12Gadget>;
    type AccountSignaturePublicKey = <Self::AccountSignatureScheme as SignatureScheme>::PublicKey;

    type EncryptedRecordCRH = BoweHopwoodPedersenCompressedCRH<EdwardsBls12, 48, 44>;
    type EncryptedRecordCRHGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls12, Self::InnerScalarField, EdwardsBls12Gadget, 48, 44>;
    type EncryptedRecordDigest = <Self::EncryptedRecordCRH as CRH>::Output;

    type EncryptionGroup = EdwardsBls12;
    type EncryptionGroupGadget = EdwardsBls12Gadget;
    type EncryptionParameters = EdwardsParameters;

    type InnerCircuitIDCRH = PoseidonCryptoHash<Self::OuterScalarField, 4, false>;
    type InnerCircuitIDCRHGadget = PoseidonCryptoHashGadget<Self::OuterScalarField, 4, false>;

    type LocalDataCommitmentScheme = PedersenCompressedCommitment<EdwardsBls12, 8, 162>;
    type LocalDataCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls12, Self::InnerScalarField, EdwardsBls12Gadget, 8, 162>;

    type LocalDataCRH = BoweHopwoodPedersenCompressedCRH<EdwardsBls12, 16, 32>;
    type LocalDataCRHGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls12, Self::InnerScalarField, EdwardsBls12Gadget, 16, 32>;
    type LocalDataDigest = <Self::LocalDataCRH as CRH>::Output;

    type PRF = Blake2s;
    type PRFGadget = Blake2sGadget;

    type ProgramCommitmentScheme = Blake2sCommitment;
    type ProgramCommitmentGadget = Blake2sCommitmentGadget;
    type ProgramCommitment = <Self::ProgramCommitmentScheme as CommitmentScheme>::Output;

    type ProgramIDCRH = PoseidonCryptoHash<Self::OuterScalarField, 4, false>;
    type ProgramIDCRHGadget = PoseidonCryptoHashGadget<Self::OuterScalarField, 4, false>;

    type RecordCommitmentScheme = PedersenCompressedCommitment<EdwardsBls12, 8, 233>;
    type RecordCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls12, Self::InnerScalarField, EdwardsBls12Gadget, 8, 233>;
    type RecordCommitment = <Self::RecordCommitmentScheme as CommitmentScheme>::Output;

    type RecordCommitmentTreeCRH = BoweHopwoodPedersenCompressedCRH<EdwardsBls12, 8, 32>;
    type RecordCommitmentTreeCRHGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls12, Self::InnerScalarField, EdwardsBls12Gadget, 8, 32>;
    type RecordCommitmentTreeDigest = <Self::RecordCommitmentTreeCRH as CRH>::Output;
    type RecordCommitmentTreeParameters = CommitmentMerkleTreeParameters;

    type SerialNumberNonceCRH = BoweHopwoodPedersenCompressedCRH<EdwardsBls12, 32, 63>;
    type SerialNumberNonceCRHGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls12, Self::InnerScalarField, EdwardsBls12Gadget, 32, 63>;

    dpc_setup!{account_commitment_scheme, ACCOUNT_COMMITMENT_SCHEME, AccountCommitmentScheme, ACCOUNT_COMMITMENT_INPUT} // TODO (howardwu): Rename to "AleoAccountCommitmentScheme0".
    dpc_setup!{account_encryption_scheme, ACCOUNT_ENCRYPTION_SCHEME, AccountEncryptionScheme, ACCOUNT_ENCRYPTION_INPUT} // TODO (howardwu): Rename to "AleoAccountEncryptionScheme0".
    dpc_setup!{account_signature_scheme, ACCOUNT_SIGNATURE_SCHEME, AccountSignatureScheme, ACCOUNT_ENCRYPTION_INPUT} // TODO (howardwu): Rename to "AleoAccountSignatureScheme0".
    dpc_setup!{encrypted_record_crh, ENCRYPTED_RECORD_CRH, EncryptedRecordCRH, "AleoEncryptedRecordCRH0"}
    dpc_setup!{inner_circuit_id_crh, INNER_CIRCUIT_ID_CRH, InnerCircuitIDCRH, "AleoInnerCircuitIDCRH0"}
    dpc_setup!{local_data_commitment_scheme, LOCAL_DATA_COMMITMENT_SCHEME, LocalDataCommitmentScheme, "AleoLocalDataCommitment0"} // TODO (howardwu): Rename to "AleoLocalDataCommitmentScheme0".
    dpc_setup!{local_data_crh, LOCAL_DATA_CRH, LocalDataCRH, "AleoLocalDataCRH0"}
    dpc_setup!{program_commitment_scheme, PROGRAM_COMMITMENT_SCHEME, ProgramCommitmentScheme, "AleoProgramIDCommitment0"} // TODO (howardwu): Rename to "AleoProgramCommitmentScheme0".
    dpc_setup!{program_id_crh, PROGRAM_ID_CRH, ProgramIDCRH, "AleoProgramIDCRH0"}
    dpc_setup!{record_commitment_scheme, RECORD_COMMITMENT_SCHEME, RecordCommitmentScheme, "AleoRecordCommitment0"} // TODO (howardwu): Rename to "AleoRecordCommitmentScheme0".
    dpc_setup!{record_commitment_tree_crh, RECORD_COMMITMENT_TREE_CRH, RecordCommitmentTreeCRH, "AleoLedgerMerkleTreeCRH0"} // TODO (howardwu): Rename to "AleoRecordCommitmentTreeCRH0".
    dpc_setup!{serial_number_nonce_crh, SERIAL_NUMBER_NONCE_CRH, SerialNumberNonceCRH, "AleoSerialNumberNonceCRH0"}

    // TODO (howardwu): TEMPORARY - Refactor this to a proper tree.
    fn record_commitment_tree_parameters() -> &'static Self::RecordCommitmentTreeParameters {
        static RECORD_COMMITMENT_TREE_PARAMETERS: OnceCell<<Testnet2Parameters as Parameters>::RecordCommitmentTreeParameters> = OnceCell::new();
        RECORD_COMMITMENT_TREE_PARAMETERS.get_or_init(|| Self::RecordCommitmentTreeParameters::from(Self::record_commitment_tree_crh().clone()))
    }
}

impl Testnet2Components for Testnet2Parameters {
    type FiatShamirRng = FiatShamirAlgebraicSpongeRng<
        Self::InnerScalarField,
        Self::OuterScalarField,
        PoseidonSponge<Self::OuterScalarField>,
    >;
    type InnerSNARKGadget = Groth16VerifierGadget<Self::InnerCurve, PairingGadget>;
    type MarlinMode = MarlinTestnet2Mode;
    type PolynomialCommitment = MarlinKZG10<Self::InnerCurve>;
    type PolynomialCommitmentCommitment =
        <Self::PolynomialCommitment as PolynomialCommitment<Self::InnerScalarField>>::Commitment;
    type PolynomialCommitmentVerifierKey =
        <Self::PolynomialCommitment as PolynomialCommitment<Self::InnerScalarField>>::VerifierKey;
    type ProgramSNARK = MarlinSNARK<
        Self::InnerScalarField,
        Self::OuterScalarField,
        Self::PolynomialCommitment,
        Self::FiatShamirRng,
        Self::MarlinMode,
        ProgramLocalData<Self>,
    >;
    type ProgramSNARKGadget = MarlinVerificationGadget<
        Self::InnerScalarField,
        Self::OuterScalarField,
        Self::PolynomialCommitment,
        MarlinKZG10Gadget<Self::InnerCurve, Self::OuterCurve, PairingGadget>,
    >;
}

// This is currently unused.
//
// use snarkvm_marlin::{FiatShamirAlgebraicSpongeRngVar, PoseidonSpongeVar};
//
// pub type FSG = FiatShamirAlgebraicSpongeRngVar<
//     Self::InnerScalarField,
//     Self::OuterScalarField,
//     PoseidonSponge<Self::OuterScalarField>,
//     PoseidonSpongeVar<Self::OuterScalarField>,
// >;
