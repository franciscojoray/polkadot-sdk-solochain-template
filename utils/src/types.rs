extern crate alloc;

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_runtime::{
    traits::{Extrinsic, },
    transaction_validity::InvalidTransaction,
};
use alloc::vec::Vec;

type Coin = u64;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
pub struct OutputRef {
    /// A hash of the transaction that created this output
    pub tx_hash: H256,
    /// The index of this output among all outputs created by the same transaction
    pub index: u32,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq, Clone, TypeInfo)]
pub struct Transaction {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

// Manually implement Encode and Decode for the Transaction type
// so that its encoding is the same as an opaque Vec<u8>.
impl Encode for Transaction {
    fn encode_to<T: parity_scale_codec::Output + ?Sized>(&self, dest: &mut T) {
        let inputs = self.inputs.encode();
        let outputs = self.outputs.encode();

        let total_len = (inputs.len() + outputs.len()) as u32;
        let size = parity_scale_codec::Compact::<u32>(total_len).encode();

        dest.write(&size);
        dest.write(&inputs);
        dest.write(&outputs);
    }
}

impl Decode for Transaction {
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        // Throw away the length of the vec. We just want the bytes.
        <parity_scale_codec::Compact<u32>>::skip(input)?;

        let inputs = <Vec<Input>>::decode(input)?;
        let outputs = <Vec<Output>>::decode(input)?;

        Ok(Transaction { inputs, outputs })
    }
}

impl Extrinsic for Transaction {
    type Call = Self;
    type SignaturePayload = ();

    fn new(data: Self, _: Option<Self::SignaturePayload>) -> Option<Self> {
        Some(data)
    }

    fn is_signed(&self) -> Option<bool> {
        None
    }
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
pub struct Input {
    /// a reference to the output being consumed
    pub output_ref: OutputRef,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PallasError {
    Error(u8),
}

// Only using the Custom variant of the enum
impl From<PallasError> for InvalidTransaction {
    fn from(error: PallasError) -> Self {
        let PallasError::Error(n) = error;

        InvalidTransaction::Custom(n)
    }
}

pub type PallasResult = Result<(), PallasError>;

#[derive(Serialize, Deserialize, Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
pub struct Output {
    pub coin: Coin,
}

impl From<Coin> for Output {
    fn from(coin: Coin) -> Self {
        Self { coin }
    }
}
