//! Helper module to build a genesis configuration for the template runtime.

#[cfg(feature = "std")]
pub use super::WASM_BINARY;
use super::{
    Transaction,
    Output
};
use griffin_core::{ensure, EXTRINSIC_KEY};
use sp_core::{H256, Encode};
use sp_std::{ vec::Vec, vec };
use hex_literal::hex;
use sp_runtime::traits::Hash;

type OutputRef = griffin_core::types::OutputRef;

/// A default seed phrase for signing inputs when none is provided
/// Corresponds to the default pubkey.
pub const SHAWN_PHRASE: &str =
    "news slush supreme milk chapter athlete soap sausage put clutch what kitten";

/// The public key corresponding to the default seed above.
pub const SHAWN_PUB_KEY: &str = "d2bf4b844dfefd6772a8843e669f943408966a977e3ae2af1dd78e0f55f4df67";

/// This function returns a list of valid transactions to be included in the genesis block.
/// It is called by the `ChainSpec::build` method, via the `development_genesis_config` function.
/// The resulting transactions must be ordered: inherent first, then extrinsics.
pub fn development_genesis_transactions() -> Vec<Transaction> {
    vec![
        Transaction {
            inputs: vec![],
            outputs: vec![
                Output {payload: 100, owner: H256::from(hex!("d2bf4b844dfefd6772a8843e669f943408966a977e3ae2af1dd78e0f55f4df67"))}
            ]
        }
    ]
}

pub fn development_genesis_config() -> serde_json::Value {
    serde_json::json!(development_genesis_transactions())
}

pub fn build(genesis_transactions: Vec<Transaction>) -> sp_genesis_builder::Result {
    // The transactions are stored under a special key.
    sp_io::storage::set(EXTRINSIC_KEY, &genesis_transactions.encode());
    
    // //TODO This was added in during merge conflicts. Make sure inherents are working even in real parachains.
    // // Initialize the stored block number to 0
    // sp_io::storage::set(HEIGHT_KEY, &0u32.encode());

    for tx in genesis_transactions.into_iter() {
        // // Enforce that inherents are in the right place
        // let current_tx_is_inherent = tx.checker.is_inherent();
        // if current_tx_is_inherent && finished_with_opening_inherents {
        //     return Err(
        //         "Tried to execute opening inherent after switching to non-inherents.".into(),
        //     );
        // }
        // if !current_tx_is_inherent && !finished_with_opening_inherents {
        //     // This is the first non-inherent, so we update our flag and continue.
        //     finished_with_opening_inherents = true;
        // }
        // Enforce that transactions do not have any inputs or peeks.
        ensure!(
            tx.inputs.is_empty(),
            "Genesis transactions must not have any inputs."
        );
        // Insert the outputs into the storage.
        let tx_hash = sp_runtime::traits::BlakeTwo256::hash_of(&tx.encode());
        for (index, utxo) in tx.outputs.iter().enumerate() {
            let output_ref = OutputRef {
                tx_hash,
                index: index as u32,
            };
            sp_io::storage::set(&output_ref.encode(), &utxo.encode());
        }
    }

    Ok(())
}
