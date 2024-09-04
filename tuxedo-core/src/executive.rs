//! # Executive Module
//!
//! The executive is the main orchestrator for the entire runtime.
//! It has functions that implement the Core, BlockBuilder, and TxPool runtime APIs.
//!
//! It does all the reusable verification of UTXO transactions such as checking that there
//! are no duplicate inputs, and that the verifiers are satisfied.

use crate::{
    // constraint_checker::ConstraintChecker,
    // dynamic_typing::DynamicallyTypedData,
    ensure,
    // inherents::PARENT_INHERENT_IDENTIFIER,
    types::{Block, BlockNumber, DispatchResult, Header, OutputRef, Transaction, UtxoError},
    utxo_set::TransparentUtxoSet,
    // verifier::Verifier,
    EXTRINSIC_KEY,
    HEADER_KEY,
    HEIGHT_KEY,
    LOG_TARGET,
};
use log::debug;
use parity_scale_codec::{Decode, Encode};
// use sp_core::H256;
// use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
    traits::{BlakeTwo256, Block as BlockT, Extrinsic, Hash as HashT, Header as HeaderT},
    transaction_validity::{
        InvalidTransaction, TransactionLongevity, TransactionSource, TransactionValidity,
        TransactionValidityError, ValidTransaction,
    },
    ApplyExtrinsicResult, ExtrinsicInclusionMode, StateVersion,
};
// use sp_std::marker::PhantomData;
use sp_std::{collections::btree_set::BTreeSet, vec::Vec};

/// The executive. Each runtime is encouraged to make a type alias called `Executive` that fills
/// in the proper generic types.
pub struct Executive;

impl Executive
where
    Block: BlockT,
    Transaction: Extrinsic,
{
    /// Does pool-style validation of a tuxedo transaction.
    /// Does not commit anything to storage.
    /// This returns Ok even if some inputs are still missing because the tagged transaction pool can handle that.
    /// We later check that there are no missing inputs in `apply_tuxedo_transaction`
    pub fn validate_tuxedo_transaction(
        transaction: &Transaction,
    ) -> Result<ValidTransaction, UtxoError> {
        debug!(
            target: LOG_TARGET,
            "validating tuxedo transaction",
        );

        // Make sure there are no duplicate inputs
        // Duplicate peeks are allowed, although they are inefficient and wallets should not create such transactions
        {
            let input_set: BTreeSet<_> = transaction.inputs.iter().map(|o| o.encode()).collect();
            ensure!(
                input_set.len() == transaction.inputs.len(),
                UtxoError::DuplicateInput
            );
        }

        // Build the stripped transaction (with the redeemers stripped) and encode it
        // This will be passed to the verifiers
        // let stripped = transaction.clone();
        // for input in stripped.inputs.iter_mut() {
        //     input.redeemer = Default::default();
        // }
        // let stripped_encoded = stripped.encode();

        // Check that the verifiers of all inputs are satisfied
        // Keep a Vec of the input data for passing to the constraint checker
        // Keep track of any missing inputs for use in the tagged transaction pool
        // let mut input_data = Vec::new();
        // let mut evicted_input_data = Vec::new();
        let mut missing_inputs = Vec::new();
        for input in transaction.inputs.iter() {
            if let Some(_input_utxo) = TransparentUtxoSet::peek_utxo(&input.output_ref) {
                // match input.redeemer {
                //     RedemptionStrategy::Redemption(ref redeemer) => {
                //         let redeemer = V::Redeemer::decode(&mut &redeemer[..])
                //             .map_err(|_| UtxoError::VerifierError)?;
                //         ensure!(
                //             input_utxo.verifier.verify(
                //                 &stripped_encoded,
                //                 Self::block_height(),
                //                 &redeemer
                //             ),
                //             UtxoError::VerifierError
                //         );
                //         input_data.push(input_utxo.payload);
                //     }
                //     RedemptionStrategy::Eviction => evicted_input_data.push(input_utxo.payload),
                // }
            } else {
                missing_inputs.push(input.output_ref.clone().encode());
            }
        }

        // // Make a Vec of the peek data for passing to the constraint checker
        // // Keep track of any missing peeks for use in the tagged transaction pool
        // // Use the same vec as previously to keep track of missing peeks
        // let mut peek_data = Vec::new();
        // for output_ref in transaction.peeks.iter() {
        //     if let Some(peek_utxo) = TransparentUtxoSet::peek_utxo(output_ref) {
        //         peek_data.push(peek_utxo.payload);
        //     } else {
        //         missing_inputs.push(output_ref.encode());
        //     }
        // }

        // Make sure no outputs already exist in storage
        let tx_hash = BlakeTwo256::hash_of(&transaction.encode());
        for index in 0..transaction.outputs.len() {
            let output_ref = OutputRef {
                tx_hash,
                index: index as u32,
            };

            debug!(
                target: LOG_TARGET,
                "Checking for pre-existing output {:?}", output_ref
            );

            ensure!(
                TransparentUtxoSet::peek_utxo(&output_ref).is_none(),
                UtxoError::PreExistingOutput
            );
        }

        // Calculate the tx-pool tags provided by this transaction, which
        // are just the encoded OutputRefs
        let provides = (0..transaction.outputs.len())
            .map(|i| {
                let output_ref = OutputRef {
                    tx_hash,
                    index: i as u32,
                };
                output_ref.encode()
            })
            .collect::<Vec<_>>();

        // If any of the inputs are missing, we cannot make any more progress
        // If they are all present, we may proceed to call the constraint checker
        if !missing_inputs.is_empty() {
            debug!(
                target: LOG_TARGET,
                "Transaction is valid but still has missing inputs. Returning early.",
            );
            return Ok(ValidTransaction {
                requires: missing_inputs,
                provides,
                priority: 0,
                longevity: TransactionLongevity::MAX,
                propagate: true,
            });
        }

        // Extract the payload data from each output
        // let output_data: Vec<DynamicallyTypedData> = transaction
        //     .outputs
        //     .iter()
        //     .map(|o| o.payload.clone())
        //     .collect();

        // // Call the constraint checker
        // transaction
        //     .checker
        //     .check(&input_data, &evicted_input_data, &peek_data, &output_data)
        //     .map_err(UtxoError::ConstraintCheckerError)?;

        // Return the valid transaction
        Ok(ValidTransaction {
            requires: Vec::new(),
            provides,
            priority: 0,
            longevity: TransactionLongevity::MAX,
            propagate: true,
        })
    }

    /// Does full verification and application of tuxedo transactions.
    /// Most of the validation happens in the call to `validate_tuxedo_transaction`.
    /// Once those checks are done we make sure there are no missing inputs and then update storage.
    pub fn apply_tuxedo_transaction(transaction: Transaction) -> DispatchResult {
        debug!(
            target: LOG_TARGET,
            "applying tuxedo transaction {:?}", transaction
        );

        // Re-do the pre-checks. These should have been done in the pool, but we can't
        // guarantee that foreign nodes to these checks faithfully, so we need to check on-chain.
        let valid_transaction = Self::validate_tuxedo_transaction(&transaction)?;

        // If there are still missing inputs, we cannot execute this,
        // although it would be valid in the pool
        ensure!(
            valid_transaction.requires.is_empty(),
            UtxoError::MissingInput
        );

        // At this point, all validation is complete, so we can commit the storage changes.
        Self::update_storage(transaction);

        Ok(())
    }

    /// Helper function to update the utxo set according to the given transaction.
    /// This function does absolutely no validation. It assumes that the transaction
    /// has already passed validation. Changes proposed by the transaction are written
    /// blindly to storage.
    fn update_storage(transaction: Transaction) {
        // Remove verified UTXOs
        for input in &transaction.inputs {
            TransparentUtxoSet::consume_utxo(&input.output_ref);
        }

        debug!(
            target: LOG_TARGET,
            "Transaction before updating storage {:?}", transaction
        );
        // Write the newly created utxos
        for (index, output) in transaction.outputs.iter().enumerate() {
            let output_ref = OutputRef {
                tx_hash: BlakeTwo256::hash_of(&transaction.encode()),
                index: index as u32,
            };
            TransparentUtxoSet::store_utxo(output_ref, output);
        }
    }

    /// A helper function that allows tuxedo runtimes to read the current block height
    pub fn block_height() -> BlockNumber {
        sp_io::storage::get(HEIGHT_KEY)
            .and_then(|d| BlockNumber::decode(&mut &*d).ok())
            .expect("A height is stored at the beginning of block one and never cleared.")
    }

    // These next three methods are for the block authoring workflow.
    // Open the block, apply zero or more extrinsics, close the block

    pub fn open_block(header: &Header) -> ExtrinsicInclusionMode {
        debug!(
            target: LOG_TARGET,
            "Entering initialize_block. header: {:?}", header
        );

        // Store the transient partial header for updating at the end of the block.
        // This will be removed from storage before the end of the block.
        sp_io::storage::set(HEADER_KEY, &header.encode());

        // Also store the height persistently so it is available when
        // performing pool validations and other off-chain runtime calls.
        sp_io::storage::set(HEIGHT_KEY, &header.number().encode());

        // Tuxedo blocks always allow user transactions.
        ExtrinsicInclusionMode::AllExtrinsics
    }

    pub fn apply_extrinsic(extrinsic: Transaction) -> ApplyExtrinsicResult {
        debug!(
            target: LOG_TARGET,
            "Entering apply_extrinsic: {:?}", extrinsic
        );

        // Append the current extrinsic to the transient list of extrinsics.
        // This will be used when we calculate the extrinsics root at the end of the block.
        let mut extrinsics = sp_io::storage::get(EXTRINSIC_KEY)
            .and_then(|d| <Vec<Vec<u8>>>::decode(&mut &*d).ok())
            .unwrap_or_default();
        extrinsics.push(extrinsic.encode());
        sp_io::storage::set(EXTRINSIC_KEY, &extrinsics.encode());

        // Now actually apply the extrinsic
        Self::apply_tuxedo_transaction(extrinsic).map_err(|e| {
            log::warn!(
                target: LOG_TARGET,
                "Tuxedo Transaction did not apply successfully: {:?}",
                e,
            );
            TransactionValidityError::Invalid(e.into())
        })?;

        Ok(Ok(()))
    }

    pub fn close_block() -> Header {
        let mut header = sp_io::storage::get(HEADER_KEY)
            .and_then(|d| Header::decode(&mut &*d).ok())
            .expect("We initialized with header, it never got mutated, qed");

        // the header itself contains the state root, so it cannot be inside the state (circular
        // dependency..). Make sure in execute block path we have the same rule.
        sp_io::storage::clear(HEADER_KEY);

        let extrinsics = sp_io::storage::get(EXTRINSIC_KEY)
            .and_then(|d| <Vec<Vec<u8>>>::decode(&mut &*d).ok())
            .unwrap_or_default();
        let extrinsics_root =
            <Header as HeaderT>::Hashing::ordered_trie_root(extrinsics, StateVersion::V0);
        sp_io::storage::clear(EXTRINSIC_KEY);
        header.set_extrinsics_root(extrinsics_root);

        let raw_state_root = &sp_io::storage::root(StateVersion::V1)[..];
        let state_root = <Header as HeaderT>::Hash::decode(&mut &raw_state_root[..]).unwrap();
        header.set_state_root(state_root);

        debug!(target: LOG_TARGET, "finalizing block {:?}", header);
        header
    }

    // This one is for the Core api. It is used to import blocks authored by foreign nodes.

    pub fn execute_block(block: Block) {
        debug!(
            target: LOG_TARGET,
            "Entering execute_block. block: {:?}", block
        );

        // Store the header. Although we don't need to mutate it, we do need to make
        // info, such as the block height, available to individual pieces. This will
        // be cleared before the end of the block
        sp_io::storage::set(HEADER_KEY, &block.header().encode());

        // Also store the height persistently so it is available when
        // performing pool validations and other off-chain runtime calls.
        sp_io::storage::set(HEIGHT_KEY, &block.header().number().encode());

        // Tuxedo requires that inherents are at the beginning (and soon end) of the
        // block and not scattered throughout. We use this flag to enforce that.
        let mut finished_with_opening_inherents = false;

        // Apply each extrinsic
        for extrinsic in block.extrinsics() {
            // Enforce that inherents are in the right place
            let current_tx_is_inherent = false; // extrinsic.checker.is_inherent();
            if current_tx_is_inherent && finished_with_opening_inherents {
                panic!("Tried to execute opening inherent after switching to non-inherents.");
            }
            if !current_tx_is_inherent && !finished_with_opening_inherents {
                // This is the first non-inherent, so we update our flag and continue.
                finished_with_opening_inherents = true;
            }

            match Self::apply_tuxedo_transaction(extrinsic.clone()) {
                Ok(()) => debug!(
                    target: LOG_TARGET,
                    "Successfully executed extrinsic: {:?}", extrinsic
                ),
                Err(e) => panic!("{:?}", e),
            }
        }

        // Clear the transient header out of storage
        sp_io::storage::clear(HEADER_KEY);

        // Check state root
        let raw_state_root = &sp_io::storage::root(StateVersion::V1)[..];
        let state_root = <Header as HeaderT>::Hash::decode(&mut &raw_state_root[..]).unwrap();
        assert_eq!(
            *block.header().state_root(),
            state_root,
            "state root mismatch"
        );

        // Check extrinsics root.
        let extrinsics = block
            .extrinsics()
            .iter()
            .map(|x| x.encode())
            .collect::<Vec<_>>();
        let extrinsics_root =
            <Header as HeaderT>::Hashing::ordered_trie_root(extrinsics, StateVersion::V0);
        assert_eq!(
            *block.header().extrinsics_root(),
            extrinsics_root,
            "extrinsics root mismatch"
        );
    }

    // This one is the pool api. It is used to make preliminary checks in the transaction pool

    pub fn validate_transaction(
        source: TransactionSource,
        tx: Transaction,
        block_hash: <Block as BlockT>::Hash,
    ) -> TransactionValidity {
        debug!(
            target: LOG_TARGET,
            "Entering validate_transaction. source: {:?}, tx: {:?}, block hash: {:?}",
            source,
            tx,
            block_hash
        );

        // Inherents are not permitted in the pool. They only come from the block author.
        // We perform this check here rather than in the `validate_tuxedo_transaction` helper,
        // because that helper is called again during on-chain execution. Inherents are valid
        // during execution, so we do not want this check repeated.
        let r = if false {
            // tx.checker.is_inherent() {
            Err(TransactionValidityError::Invalid(InvalidTransaction::Call))
        } else {
            Self::validate_tuxedo_transaction(&tx).map_err(|e| {
                log::warn!(
                    target: LOG_TARGET,
                    "Tuxedo Transaction did not validate (in the pool): {:?}",
                    e,
                );
                TransactionValidityError::Invalid(e.into())
            })
        };

        debug!(target: LOG_TARGET, "Validation result: {:?}", r);

        r
    }

    // // The next two are for the standard beginning-of-block inherent extrinsics.
    // pub fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<Transaction> {
    //     debug!(
    //         target: LOG_TARGET,
    //         "Entering `inherent_extrinsics`."
    //     );
    //
    //     // Extract the complete parent block from the inherent data
    //     let parent: Block = data
    //         .get_data(&PARENT_INHERENT_IDENTIFIER)
    //         .expect("Parent block inherent data should be able to decode.")
    //         .expect("Parent block should be present among authoring inherent data.");
    //
    //     // Extract the inherents from the previous block, which can be found at the beginning of the extrinsics list.
    //     // The parent is already imported, so we know it is valid and we know its inherents came first.
    //     // We also annotate each transaction with its original hash for purposes of constructing output refs later.
    //     // This is necessary because the transaction hash changes as we unwrap layers of aggregation,
    //     // and we need an original universal transaction id.
    //     let previous_blocks_inherents: Vec<(Transaction, H256)> = parent
    //         .extrinsics()
    //         .iter()
    //         .cloned()
    //         .take_while(|tx| tx.checker.is_inherent())
    //         .map(|tx| {
    //             let id = BlakeTwo256::hash_of(&tx.encode());
    //             (tx, id)
    //         })
    //         .collect();
    //
    //     debug!(
    //         target: LOG_TARGET,
    //         "The previous block had {} extrinsics ({} inherents).", parent.extrinsics().len(), previous_blocks_inherents.len()
    //     );
    //
    //     // Call into constraint checker's own inherent hooks to create the actual transactions
    //     C::create_inherents(&data, previous_blocks_inherents)
    // }
    //
    // pub fn check_inherents(
    //     block: Block,
    //     data: InherentData,
    // ) -> sp_inherents::CheckInherentsResult {
    //     debug!(
    //         target: LOG_TARGET,
    //         "Entering `check_inherents`"
    //     );
    //
    //     let mut result = CheckInherentsResult::new();
    //
    //     // Tuxedo requires that all inherents come at the beginning of the block.
    //     // (Soon we will also allow them at the end, but never throughout the body.)
    //     // (TODO revise this logic once that is implemented.)
    //     // At this off-chain pre-check stage, we assume that requirement is upheld.
    //     // It will be verified later once we are executing on-chain.
    //     let inherents: Vec<Transaction> = block
    //         .extrinsics()
    //         .iter()
    //         .cloned()
    //         .take_while(|tx| tx.checker.is_inherent())
    //         .collect();
    //
    //     C::check_inherents(&data, inherents, &mut result);
    //
    //     result
    // }
}
