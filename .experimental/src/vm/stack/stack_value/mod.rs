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

use console::{
    network::prelude::*,
    program::{Plaintext, Record},
};

#[derive(Clone, PartialEq, Eq)]
pub enum StackValue<N: Network> {
    /// A plaintext value.
    Plaintext(Plaintext<N>),
    /// A record value.
    Record(Record<N, Plaintext<N>>),
}

impl<N: Network> StackValue<N> {
    /// Returns the stack value as a list of **little-endian** bits.
    #[inline]
    pub fn to_bits_le(&self) -> Vec<bool> {
        match self {
            StackValue::Plaintext(Plaintext::Literal(literal, ..)) => {
                [literal.variant().to_bits_le(), literal.to_bits_le()].into_iter().flatten().collect()
            }
            StackValue::Plaintext(Plaintext::Interface(interface, ..)) => interface
                .into_iter()
                .flat_map(|(member_name, member_value)| {
                    [member_name.to_bits_le(), member_value.to_bits_le()].into_iter().flatten()
                })
                .collect(),
            StackValue::Record(record) => record
                .owner()
                .to_bits_le()
                .into_iter()
                .chain(record.balance().to_bits_le().into_iter())
                .chain(record.data().iter().flat_map(|(entry_name, entry_value)| {
                    [entry_name.to_bits_le(), entry_value.to_bits_le()].into_iter().flatten()
                }))
                .collect(),
        }
    }
}
