//! Wallet features related to spending money and checking balances.

use crate::{cli::MintCoinArgs, cli::SpendArgs, rpc::fetch_storage, sync};

use anyhow::anyhow;
use jsonrpsee::{core::client::ClientT, http_client::HttpClient, rpc_params};
use parity_scale_codec::Encode;
use sc_keystore::LocalKeystore;
use sled::Db;
use sp_core::H256; // {sr25519::Public, H256};
use sp_runtime::traits::{BlakeTwo256, Hash};
use tuxedo_core::{
    types::{Coin, Input, Output, OutputRef, Transaction},
};

/// Create and send a transaction that mints the coins on the network
pub async fn mint_coins(
    parachain: bool,
    client: &HttpClient,
    args: MintCoinArgs,
) -> anyhow::Result<()> {
    if parachain {
        mint_coins_helper(client, args).await
    } else {
        mint_coins_helper(client, args).await
    }
}

pub async fn mint_coins_helper(client: &HttpClient, args: MintCoinArgs) -> anyhow::Result<()> {
    log::debug!("The args are:: {:?}", args);

    let transaction: tuxedo_core::types::Transaction = Transaction {
        inputs: Vec::new(),
        outputs: vec![Output {
            payload: args.amount,
        }],
    };

    let encoded_tx = hex::encode(transaction.encode());
    let params = rpc_params![encoded_tx];
    let _spawn_response: Result<String, _> = client.request("author_submitExtrinsic", params).await;

    log::info!(
        "Node's response to mint-coin transaction: {:?}",
        _spawn_response
    );

    let minted_coin_ref = OutputRef {
        tx_hash: <BlakeTwo256 as Hash>::hash_of(&transaction.encode()),
        index: 0,
    };
    let output = &transaction.outputs[0];
    let amount = output.payload;
    println!(
        "Minted {:?} worth {amount}. ",
        hex::encode(minted_coin_ref.encode())
    );
    // crate::pretty_print_verifier(&output.verifier);

    Ok(())
}

/// Create and send a transaction that spends coins on the network
pub async fn spend_coins(
    parachain: bool,
    db: &Db,
    client: &HttpClient,
    keystore: &LocalKeystore,
    args: SpendArgs,
) -> anyhow::Result<()> {
    // Depending how the parachain and metadata support shapes up, it may make sense to have a
    // macro that writes all of these helpers and ifs.
    if parachain {
        spend_coins_helper(db, client, keystore, args).await
    } else {
        spend_coins_helper(db, client, keystore, args).await
    }
}

pub async fn spend_coins_helper(
    db: &Db,
    client: &HttpClient,
    _keystore: &LocalKeystore,
    args: SpendArgs,
) -> anyhow::Result<()> {
    log::debug!("The args are:: {:?}", args);

    // Construct a template Transaction to push coins into later
    let mut transaction: Transaction = Transaction {
        inputs: Vec::new(),
        // peeks: Vec::new(),
        outputs: Vec::new(),
        // checker: OuterConstraintChecker::Money(MoneyConstraintChecker::Spend).into(),
    };

    // Construct each output and then push to the transactions
    let mut total_output_amount: u64 = 0;
    for amount in &args.output_amount {
        let output = Output {
            payload: *amount,
            // verifier: OuterVerifier::Sr25519Signature(Sr25519Signature {
            // owner_pubkey: args.recipient,
            // }),
        };
        total_output_amount += *amount;
        transaction.outputs.push(output);
    }

    // The total input set will consist of any manually chosen inputs
    // plus any automatically chosen to make the input amount high enough
    let mut total_input_amount: u64 = 0;
    let mut all_input_refs = args.input;
    for output_ref in &all_input_refs {
        let (_owner_pubkey, amount) = sync::get_unspent(db, output_ref)?.ok_or(anyhow!(
            "user-specified output ref not found in local database"
        ))?;
        total_input_amount += amount;
    }
    //TODO filtering on a specific sender

    // If the supplied inputs are not valuable enough to cover the output amount
    // we select the rest arbitrarily from the local db. (In many cases, this will be all the inputs.)
    if total_input_amount < total_output_amount {
        match sync::get_arbitrary_unspent_set(db, total_output_amount - total_input_amount)? {
            Some(more_inputs) => {
                all_input_refs.extend(more_inputs);
            }
            None => Err(anyhow!(
                "Not enough value in database to construct transaction"
            ))?,
        }
    }

    // Make sure each input decodes and is still present in the node's storage,
    // and then push to transaction.
    for output_ref in &all_input_refs {
        get_coin_from_storage(output_ref, client).await?;
        transaction.inputs.push(Input {
            output_ref: output_ref.clone(),
            // redeemer: Default::default(), // We will sign the total transaction so this should be empty
        });
    }

    // Keep a copy of the stripped encoded transaction for signing purposes
    // let stripped_encoded_transaction = transaction.clone().encode();

    // Iterate back through the inputs, signing, and putting the signatures in place.
    // for input in &mut transaction.inputs {
    // Fetch the output from storage
    // let utxo = fetch_storage(&input.output_ref, client).await?;

    // // Construct the proof that it can be consumed
    // let redeemer = match utxo.verifier {
    //     OuterVerifier::Sr25519Signature(Sr25519Signature { owner_pubkey }) => {
    //         let public = Public::from_h256(owner_pubkey);
    //         let signature =
    //             crate::keystore::sign_with(keystore, &public, &stripped_encoded_transaction)?;
    //         OuterVerifierRedeemer::Sr25519Signature(signature)
    //     }
    //     OuterVerifier::UpForGrabs(_) => OuterVerifierRedeemer::UpForGrabs(()),
    //     OuterVerifier::ThresholdMultiSignature(_) => todo!(),
    // };

    // // insert the proof
    // let encoded_redeemer = redeemer.encode();
    // log::debug!("encoded redeemer is: {:?}", encoded_redeemer);
    //
    // input.redeemer = RedemptionStrategy::Redemption(encoded_redeemer);
    // }

    log::debug!("signed transactions is: {:#?}", transaction);

    // Send the transaction
    let genesis_spend_hex = hex::encode(transaction.encode());
    let params = rpc_params![genesis_spend_hex];
    let genesis_spend_response: Result<String, _> =
        client.request("author_submitExtrinsic", params).await;
    log::info!(
        "Node's response to spend transaction: {:?}",
        genesis_spend_response
    );

    // Print new output refs for user to check later
    let tx_hash = <BlakeTwo256 as Hash>::hash_of(&transaction.encode());
    for (i, output) in transaction.outputs.iter().enumerate() {
        let new_coin_ref = OutputRef {
            tx_hash,
            index: i as u32,
        };
        let amount = output.payload;

        print!(
            "Created {:?} worth {amount}. ",
            hex::encode(new_coin_ref.encode())
        );
        // crate::pretty_print_verifier(&output.verifier);
    }

    Ok(())
}

/// Given an output ref, fetch the details about this coin from the node's
/// storage.
pub async fn get_coin_from_storage(
    output_ref: &OutputRef,
    client: &HttpClient,
) -> anyhow::Result<Coin> {
    let utxo = fetch_storage(output_ref, client).await?;
    let coin_in_storage: Coin = utxo.payload;

    Ok(coin_in_storage)
}

/// Apply a transaction to the local database, storing the new coins.
pub(crate) fn apply_transaction(
    db: &Db,
    tx_hash: <BlakeTwo256 as Hash>::Output,
    index: u32,
    output: &Output,
) -> anyhow::Result<()> {
    let amount = output.payload;
    let output_ref = OutputRef { tx_hash, index };
    let owner_pubkey = H256::from_slice(b"                                ");
    crate::sync::add_unspent_output(db, &output_ref, &owner_pubkey, &amount)
}
