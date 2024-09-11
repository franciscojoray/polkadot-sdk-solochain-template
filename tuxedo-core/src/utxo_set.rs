//! For now this is a higher level description of the default UTXO set features which Tuxedo
//! chooses to use. Future UTXO sets could take a different form especially
//! if being used for Zero-Knowledge. In the future it may likely be abstracted into a trait
//! to support various UTXO set types.
//!

use crate::{
    types::{Output, OutputRef},
    LOG_TARGET,
};
use parity_scale_codec::{Decode, Encode};

pub struct TransparentUtxoSet;

impl TransparentUtxoSet {
    /// Fetch a utxo from the set.
    pub fn peek_utxo(output_ref: &OutputRef) -> Option<Output> {
        sp_io::storage::get(&output_ref.encode()).and_then(|d| Output::decode(&mut &*d).ok())
    }

    /// Consume a Utxo from the set.
    pub fn consume_utxo(output_ref: &OutputRef) -> Option<Output> {
        // TODO do we even need to read the stored value here? The only place we call this
        // is from `update_storage` and we don't use the value there.
        let maybe_output = Self::peek_utxo(output_ref);
        sp_io::storage::clear(&output_ref.encode());
        maybe_output
    }

    /// Add a utxo into the set.
    /// This will overwrite any utxo that already exists at this OutputRef. It should never be the
    /// case that there are collisions though. Right??
    pub fn store_utxo(output_ref: OutputRef, output: &Output) {
        let key = output_ref.encode();
        log::debug!(
            target: LOG_TARGET,
            "Storing UTXO at key: {:?}",
            sp_core::hexdisplay::HexDisplay::from(&key)
        );
        sp_io::storage::set(&key, &output.encode());
    }
}
