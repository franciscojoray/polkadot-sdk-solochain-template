use crate::{
    types::{Output, OutputRef},
};
use parity_scale_codec::{Decode, Encode};
use sp_io::storage;

pub fn get_utxo(output_ref: &OutputRef) -> Option<Output> {
    storage::get(&output_ref.encode()).and_then(|d| Output::decode(&mut &*d).ok())
}

pub fn consume_utxo(output_ref: &OutputRef) -> Option<Output> {
    let output = Self::peek_utxo(output_ref);
    storage::clear(&output_ref.encode());

    output
}

pub fn store_utxo(output_ref: OutputRef, output: &Output) {
    let key = output_ref.encode();
    storage::set(&key, &output.encode());
}
