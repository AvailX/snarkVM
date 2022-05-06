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

use super::*;

impl<E: Environment> Ternary for Boolean<E> {
    type Boolean = Boolean<E>;
    type Output = Self;

    /// Returns `first` if `condition` is `true`, otherwise returns `second`.
    fn ternary(condition: &Self::Boolean, first: &Self, second: &Self) -> Self::Output {
        // Constant `condition`
        if condition.is_constant() {
            match condition.eject_value() {
                true => first.clone(),
                false => second.clone(),
            }
        }
        // Constant `first`
        else if first.is_constant() {
            match first.eject_value() {
                true => condition | second,
                false => !condition & second,
            }
        }
        // Constant `second`
        else if second.is_constant() {
            match second.eject_value() {
                true => !condition | first,
                false => condition & first,
            }
        }
        // Variables
        else {
            // Compute the witness value, based on the condition.
            let witness = match condition.eject_value() {
                true => first.eject_value(),
                false => second.eject_value(),
            };

            // Declare a new variable with the expected output as witness.
            // Note: The constraint below will ensure `output` is either 0 or 1,
            // assuming `self` and `other` are well-formed (they are either 0 or 1).
            let output = Boolean(
                E::new_variable(Mode::Private, match witness {
                    true => E::BaseField::one(),
                    false => E::BaseField::zero(),
                })
                .into(),
            );

            //
            // Ternary Enforcement
            // -------------------------------------------------------
            //    output = condition * a + (1 - condition) * b
            // => output = b + condition * (a - b)
            // => condition * (a - b) = output - b
            //
            // See `Field::ternary()` for the proof of correctness.
            //
            E::enforce(|| (condition, (&first.0 - &second.0), (&output.0 - &second.0)));

            output
        }
    }
}

impl<E: Environment> Metadata<dyn Ternary<Boolean = Boolean<E>, Output = Boolean<E>>> for Boolean<E> {
    type Case = (CircuitType<Boolean<E>>, CircuitType<Boolean<E>>, CircuitType<Boolean<E>>);
    type OutputType = CircuitType<Boolean<E>>;

    fn count(case: &Self::Case) -> Count {
        match case {
            (CircuitType::Constant(_), _, _) => Count::is(0, 0, 0, 0),
            (_, _, _) => Count::is(0, 0, 1, 1),
        }
    }

    fn output_type(case: Self::Case) -> Self::OutputType {
        match case {
            (CircuitType::Constant(constant), _, _) => match constant.eject_value() {
                true => case.1,
                false => case.2,
            },
            (condition_type, CircuitType::Constant(constant), second_type) => match constant.eject_value() {
                true => output_type!(Self, BitOr<Self, Output = Self>, (condition_type, second_type)),
                false => {
                    let not_output_type = output_type!(Self, Not<Output = Self>, condition_type);
                    output_type!(Self, BitAnd<Self, Output = Self>, (not_output_type, second_type))
                }
            },
            (condition_type, first_type, CircuitType::Constant(constant)) => match constant.eject_value() {
                true => {
                    let not_output_type = output_type!(Self, Not<Output = Self>, condition_type);
                    output_type!(Self, BitOr<Self, Output = Self>, (not_output_type, first_type))
                }
                false => output_type!(Self, BitAnd<Self, Output = Self>, (condition_type, first_type)),
            },
            (_, _, _) => CircuitType::Private,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm_circuits_environment::Circuit;

    fn check_ternary(
        name: &str,
        expected: bool,
        condition: Boolean<Circuit>,
        a: Boolean<Circuit>,
        b: Boolean<Circuit>,
        num_constants: u64,
        num_public: u64,
        num_private: u64,
        num_constraints: u64,
    ) {
        Circuit::scope(name, || {
            let case = format!("({} ? {} : {})", condition.eject_value(), a.eject_value(), b.eject_value());
            let candidate = Boolean::ternary(&condition, &a, &b);
            assert_eq!(expected, candidate.eject_value(), "{case}");
            assert_scope!(num_constants, num_public, num_private, num_constraints);
        });
    }

    #[test]
    fn test_constant_condition() {
        // false ? Constant : Constant
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Constant, false);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("false ? Constant : Constant", expected, condition, a, b, 0, 0, 0, 0);

        // false ? Constant : Public
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Constant, false);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("false ? Constant : Public", expected, condition, a, b, 0, 0, 0, 0);

        // false ? Public : Constant
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Constant, false);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("false ? Public : Constant", expected, condition, a, b, 0, 0, 0, 0);

        // false ? Public : Public
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Constant, false);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("false ? Public : Public", expected, condition, a, b, 0, 0, 0, 0);

        // false ? Public : Private
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Constant, false);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("false ? Public : Private", expected, condition, a, b, 0, 0, 0, 0);

        // false ? Private : Private
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Constant, false);
        let a = Boolean::<Circuit>::new(Mode::Private, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("false ? Private : Private", expected, condition, a, b, 0, 0, 0, 0);

        // true ? Constant : Constant
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Constant, true);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("true ? Constant : Constant", expected, condition, a, b, 0, 0, 0, 0);

        // true ? Constant : Public
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Constant, true);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("true ? Constant : Public", expected, condition, a, b, 0, 0, 0, 0);

        // true ? Public : Constant
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Constant, true);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("true ? Public : Constant", expected, condition, a, b, 0, 0, 0, 0);

        // true ? Public : Public
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Constant, true);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("true ? Public : Public", expected, condition, a, b, 0, 0, 0, 0);

        // true ? Public : Private
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Constant, true);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("true ? Public : Private", expected, condition, a, b, 0, 0, 0, 0);

        // true ? Private : Private
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Constant, true);
        let a = Boolean::<Circuit>::new(Mode::Private, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("true ? Private : Private", expected, condition, a, b, 0, 0, 0, 0);
    }

    #[test]
    fn test_public_condition_and_constant_input() {
        // false ? Constant : Constant
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Public, false);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("false ? Constant : Constant", expected, condition, a, b, 0, 0, 0, 0);

        // false ? Constant : Public
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Public, false);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("false ? Constant : Public", expected, condition, a, b, 0, 0, 1, 1);

        // false ? Public : Constant
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Public, false);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("false ? Public : Constant", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Constant : Constant
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Public, true);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("true ? Constant : Constant", expected, condition, a, b, 0, 0, 0, 0);

        // true ? Constant : Public
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Public, true);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("true ? Constant : Public", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Public : Constant
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Public, true);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("true ? Public : Constant", expected, condition, a, b, 0, 0, 1, 1);
    }

    #[test]
    fn test_private_condition_and_constant_input() {
        // false ? Constant : Constant
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Private, false);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("false ? Constant : Constant", expected, condition, a, b, 0, 0, 0, 0);

        // false ? Constant : Public
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Private, false);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("false ? Constant : Public", expected, condition, a, b, 0, 0, 1, 1);

        // false ? Public : Constant
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Private, false);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("false ? Public : Constant", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Constant : Constant
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Private, true);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("true ? Constant : Constant", expected, condition, a, b, 0, 0, 0, 0);

        // true ? Constant : Public
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Private, true);
        let a = Boolean::<Circuit>::new(Mode::Constant, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("true ? Constant : Public", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Public : Constant
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Private, true);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Constant, false);
        check_ternary("true ? Public : Constant", expected, condition, a, b, 0, 0, 1, 1);
    }

    #[test]
    fn test_public_condition_and_variable_inputs() {
        // false ? Public : Public
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Public, false);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("false ? Public : Public", expected, condition, a, b, 0, 0, 1, 1);

        // false ? Public : Private
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Public, false);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("false ? Public : Private", expected, condition, a, b, 0, 0, 1, 1);

        // false ? Private : Public
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Public, false);
        let a = Boolean::<Circuit>::new(Mode::Private, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("false ? Private : Public", expected, condition, a, b, 0, 0, 1, 1);

        // false ? Private : Private
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Public, false);
        let a = Boolean::<Circuit>::new(Mode::Private, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("false ? Private : Private", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Public : Public
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Public, true);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("true ? Public : Public", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Public : Private
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Public, true);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("true ? Public : Private", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Private : Public
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Public, true);
        let a = Boolean::<Circuit>::new(Mode::Private, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("true ? Private : Public", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Private : Private
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Public, true);
        let a = Boolean::<Circuit>::new(Mode::Private, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("true ? Private : Private", expected, condition, a, b, 0, 0, 1, 1);
    }

    #[test]
    fn test_private_condition_and_variable_inputs() {
        // false ? Public : Public
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Private, false);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("false ? Public : Public", expected, condition, a, b, 0, 0, 1, 1);

        // false ? Public : Private
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Private, false);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("false ? Public : Private", expected, condition, a, b, 0, 0, 1, 1);

        // false ? Private : Public
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Private, false);
        let a = Boolean::<Circuit>::new(Mode::Private, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("false ? Private : Public", expected, condition, a, b, 0, 0, 1, 1);

        // false ? Private : Private
        let expected = false;
        let condition = Boolean::<Circuit>::new(Mode::Private, false);
        let a = Boolean::<Circuit>::new(Mode::Private, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("false ? Private : Private", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Public : Public
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Private, true);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("true ? Public : Public", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Public : Private
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Private, true);
        let a = Boolean::<Circuit>::new(Mode::Public, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("true ? Public : Private", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Private : Public
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Private, true);
        let a = Boolean::<Circuit>::new(Mode::Private, true);
        let b = Boolean::<Circuit>::new(Mode::Public, false);
        check_ternary("true ? Private : Public", expected, condition, a, b, 0, 0, 1, 1);

        // true ? Private : Private
        let expected = true;
        let condition = Boolean::<Circuit>::new(Mode::Private, true);
        let a = Boolean::<Circuit>::new(Mode::Private, true);
        let b = Boolean::<Circuit>::new(Mode::Private, false);
        check_ternary("true ? Private : Private", expected, condition, a, b, 0, 0, 1, 1);
    }
}
