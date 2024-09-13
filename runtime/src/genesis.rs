//! Helper module to build a genesis configuration for the template runtime.

#[cfg(feature = "std")]
pub use super::WASM_BINARY;
use super::{
    Transaction,
    Output
};
use griffin_core::{ensure, EXTRINSIC_KEY, HEIGHT_KEY};
use sp_core::{H256, Encode};
use sp_std::{ vec::Vec, vec };
use hex::FromHex;
use sp_runtime::traits::Hash;

// DEPENDENCIAS DE BLOCK_BUILDER:
use sc_chain_spec::BuildGenesisBlock;
use sc_client_api::backend::{Backend, BlockImportOperation};
use sc_executor::RuntimeVersionOf;
use sp_core::traits::CodeExecutor;
use sp_runtime::{
    traits::{Block as BlockT, Hash as HashT, HashingFor, Header as HeaderT, Zero},
    BuildStorage,
};
use std::sync::Arc;


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
                Output {
                    payload: 314,
                    owner: H256::from(<[u8; 32]>::from_hex(SHAWN_PUB_KEY).unwrap())
                }
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
    sp_io::storage::set(HEIGHT_KEY, &0u32.encode());

    for tx in genesis_transactions.into_iter() {
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

pub struct GriffinGenesisBlockBuilder<
    'a,
    Block: BlockT,
    B: Backend<Block>,
    E: RuntimeVersionOf + CodeExecutor,
> {
    build_genesis_storage: &'a dyn BuildStorage,
    commit_genesis_state: bool,
    backend: Arc<B>,
    executor: E,
    _phantom: std::marker::PhantomData<Block>,
}

impl<'a, Block: BlockT, B: Backend<Block>, E: RuntimeVersionOf + CodeExecutor>
    GriffinGenesisBlockBuilder<'a, Block, B, E>
{
    pub fn new(
        build_genesis_storage: &'a dyn BuildStorage,
        commit_genesis_state: bool,
        backend: Arc<B>,
        executor: E,
    ) -> sp_blockchain::Result<Self> {
        Ok(Self {
            build_genesis_storage,
            commit_genesis_state,
            backend,
            executor,
            _phantom: Default::default(),
        })
    }
}

impl<'a, Block: BlockT, B: Backend<Block>, E: RuntimeVersionOf + CodeExecutor>
    BuildGenesisBlock<Block> for GriffinGenesisBlockBuilder<'a, Block, B, E>
{
    type BlockImportOperation = <B as Backend<Block>>::BlockImportOperation;

    /// Build the genesis block, including the extrinsics found in storage at EXTRINSIC_KEY.
    /// The extrinsics are not checked for validity, nor executed, so the values in storage must be placed manually.
    /// This can be done by using the `assimilate_storage` function.
    fn build_genesis_block(self) -> sp_blockchain::Result<(Block, Self::BlockImportOperation)> {
        // We build it here to gain mutable access to the storage.
        let mut genesis_storage = self
            .build_genesis_storage
            .build_storage()
            .map_err(sp_blockchain::Error::Storage)?;

        let state_version = sc_chain_spec::resolve_state_version_from_wasm::<_, HashingFor<Block>>(
            &genesis_storage,
            &self.executor,
        )?;

        let extrinsics = match genesis_storage.top.remove(crate::EXTRINSIC_KEY) {
            Some(v) => <Vec<<Block as BlockT>::Extrinsic>>::decode(&mut &v[..]).unwrap_or_default(),
            None => Vec::new(),
        };

        let extrinsics_root =
            <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::ordered_trie_root(
                extrinsics.iter().map(codec::Encode::encode).collect(),
                state_version,
            );

        let mut op = self.backend.begin_operation()?;
        let state_root =
            op.set_genesis_state(genesis_storage, self.commit_genesis_state, state_version)?;

        let block = Block::new(
            HeaderT::new(
                Zero::zero(),
                extrinsics_root,
                state_root,
                Default::default(),
                Default::default(),
            ),
            extrinsics,
        );

        Ok((block, op))
    }
}

