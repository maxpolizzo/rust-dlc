#[path = "../../dlc-manager/tests/test_utils.rs"]
mod test_utils;

use bitcoin::{Amount, Network};
use bitcoin_rpc_provider::BitcoinCoreProvider;
use bitcoin_test_utils::rpc_helpers::get_new_wallet_rpc;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use bitcoincore_rpc_json::AddressType;
use colored::Colorize;
use dlc_manager::manager::Manager;
use dlc_manager::{Oracle, Wallet};
use dlc_messages::{AcceptDlc, Message, OfferDlc, SignDlc, ACCEPT_TYPE, OFFER_TYPE, SIGN_TYPE};
use lightning::util::ser::Writeable;
use mocks::memory_storage_provider::MemoryStorage;
use simple_wallet::SimpleWallet;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use test_utils::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DlcOfferMessage {
    message: OfferDlc,
    serialized: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DlcAcceptMessage {
    message: AcceptDlc,
    serialized: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DlcSignMessage {
    message: SignDlc,
    serialized: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DlcTestVector {
    offer_message: DlcOfferMessage,
    accept_message: DlcAcceptMessage,
    sign_message: DlcSignMessage,
}

fn main() {
    println!("\n{}\n", "Generating DLC test vectors...".bold().yellow());
    // Instantiate RPC clients
    let (offer_client, accept_client, miner_client, rpc_client) =
        init_clients("http://localhost:18443/", "lnd", "lightning");
    // Instantiate RPC providers
    let rpc_provider = BitcoinCoreProvider::new_from_rpc_client(rpc_client);
    let alice_provider = BitcoinCoreProvider::new_from_rpc_client(offer_client);
    let bob_provider = BitcoinCoreProvider::new_from_rpc_client(accept_client);
    // Instantiate wallets for DLC offer party "Alice" and DLC accept party "Bob"
    let (
        alice_wallet,
        bob_wallet,
        alice_store,
        bob_store
    ) = init_wallets(&alice_provider, &bob_provider);
    // Generate test params
    let (test_params, test_vectors_names) = generate_test_params();
    // Iterate over test params to generate test vectors
    let blockchain = Arc::new(rpc_provider);
    for (i, test_param) in test_params.into_iter().enumerate() {
        // Fund new UTXOs for Alice and Bob
        fund_utxos(
            Arc::clone(&alice_wallet),
            Arc::clone(&bob_wallet),
            &miner_client,
        );
        generate_test_vector(
            Arc::clone(&blockchain),
            Arc::clone(&alice_wallet),
            Arc::clone(&bob_wallet),
            Arc::clone(&alice_store),
            Arc::clone(&bob_store),
            test_param,
            test_vectors_names[i]
        );
    }
    // Done
    println!("\n{}\n", "Done.".bold().yellow());
}

fn generate_test_params() -> (Vec<TestParams>, Vec<& 'static str>) {
    // Generate test params and oracles announcements
    let mut test_params: Vec<TestParams> = Vec::new();
    let mut test_vectors_names: Vec<&str> = Vec::new();
    // Enumerated outcomes, 1 oracle
    test_vectors_names.push("enum_single_oracle");
    test_params.push(get_enum_test_params(1, 1, None));
    // Enumerated outcomes, 3 of 3 oracles
    test_vectors_names.push("enum_3_of_3_oracles");
    test_params.push(get_enum_test_params(3, 3, None));
    // Enumerated outcomes, 3 of 5 oracles
    test_vectors_names.push("enum_3_of_5_oracles");
    test_params.push(get_enum_test_params(5,3, None));
    // Numerical outcome (hyperbola), 1 oracle
    let mut nb_oracles = 1;
    let mut oracle_numeric_infos = get_same_num_digits_oracle_numeric_infos(nb_oracles);
    let mut numerical_contract_descriptor = get_numerical_contract_descriptor(
        get_same_num_digits_oracle_numeric_infos(nb_oracles),
        get_hyperbola_payout_curve_pieces(oracle_numeric_infos.get_min_nb_digits()),
        None,
    );
    test_vectors_names.push("numerical_hyperbola_1_oracle");
    test_params.push(
        get_numerical_test_params(
            &oracle_numeric_infos,
            1,
            false,
            numerical_contract_descriptor,
            true
        )
    );
    // Numerical outcome (polynomial), 1 oracle
    nb_oracles = 1;
    oracle_numeric_infos = get_same_num_digits_oracle_numeric_infos(nb_oracles);
    numerical_contract_descriptor = get_numerical_contract_descriptor(
        get_same_num_digits_oracle_numeric_infos(nb_oracles),
        get_polynomial_payout_curve_pieces(oracle_numeric_infos.get_min_nb_digits()),
        None,
    );
    test_vectors_names.push("numerical_polynomial_1_oracle");
    test_params.push(
        get_numerical_test_params(
            &oracle_numeric_infos,
            1,
            false,
            numerical_contract_descriptor,
            true)
    );
    // Numerical outcome (polynomial), 3 of 3 oracles
    nb_oracles = 3;
    oracle_numeric_infos = get_same_num_digits_oracle_numeric_infos(nb_oracles);
    numerical_contract_descriptor = get_numerical_contract_descriptor(
        get_same_num_digits_oracle_numeric_infos(nb_oracles),
        get_polynomial_payout_curve_pieces(oracle_numeric_infos.get_min_nb_digits()),
        None,
    );
    test_vectors_names.push("numerical_polynomial_3_of_3_oracles");
    test_params.push(get_numerical_test_params(&oracle_numeric_infos, 3, false, numerical_contract_descriptor, true));
    // Numerical outcome (polynomial) with diff, 3 of 3 oracles
    nb_oracles = 3;
    oracle_numeric_infos = get_same_num_digits_oracle_numeric_infos(nb_oracles);
    numerical_contract_descriptor = get_numerical_contract_descriptor(
        get_same_num_digits_oracle_numeric_infos(nb_oracles),
        get_polynomial_payout_curve_pieces(oracle_numeric_infos.get_min_nb_digits()),
        None,
    );
    test_vectors_names.push("numerical_polynomial_3_of_3_oracles_with_diff");
    test_params.push(get_numerical_test_params(&oracle_numeric_infos, 3, true, numerical_contract_descriptor, true));
    // Numerical outcome (polynomial) with diff, 3 of 5 oracles
    nb_oracles = 5;
    oracle_numeric_infos = get_same_num_digits_oracle_numeric_infos(nb_oracles);
    numerical_contract_descriptor = get_numerical_contract_descriptor(
        get_same_num_digits_oracle_numeric_infos(nb_oracles),
        get_polynomial_payout_curve_pieces(oracle_numeric_infos.get_min_nb_digits()),
        None,
    );
    test_vectors_names.push("numerical_polynomial_3_of_5_oracles_with_diff");
    test_params.push(get_numerical_test_params(&oracle_numeric_infos, 3, true, numerical_contract_descriptor, true));
    //
    // Numerical outcome (polynomial), 2 of 5 oracles
    nb_oracles = 5;
    oracle_numeric_infos = get_same_num_digits_oracle_numeric_infos(nb_oracles);
    numerical_contract_descriptor = get_numerical_contract_descriptor(
        get_same_num_digits_oracle_numeric_infos(nb_oracles),
        get_polynomial_payout_curve_pieces(oracle_numeric_infos.get_min_nb_digits()),
        None,
    );
    test_vectors_names.push("numerical_polynomial_2_of_5_oracles");
    test_params.push(get_numerical_test_params(&oracle_numeric_infos, 2, false, numerical_contract_descriptor, true));
    // Numerical outcome (polynomial) with diff, 2 of 5 oracles
    nb_oracles = 5;
    oracle_numeric_infos = get_same_num_digits_oracle_numeric_infos(nb_oracles);
    numerical_contract_descriptor = get_numerical_contract_descriptor(
        get_same_num_digits_oracle_numeric_infos(nb_oracles),
        get_polynomial_payout_curve_pieces(oracle_numeric_infos.get_min_nb_digits()),
        None,
    );
    test_vectors_names.push("numerical_polynomial_2_of_5_oracles_with_diff");
    test_params.push(get_numerical_test_params(&oracle_numeric_infos, 2, true, numerical_contract_descriptor, true));
    // Enumerated and numerical outcomes, 3 of 5 oracles
    test_vectors_names.push("enum_and_numerical_polynomial_3_of_5_oracles_with_diff");
    test_params.push(get_enum_and_numerical_test_params(5,3, true, None));
    // Enumerated and numerical outcomes, 5 of 5 oracles
    test_vectors_names.push("enum_and_numerical_polynomial_5_of_5_oracles_with_diff");
    test_params.push(get_enum_and_numerical_test_params(5,5, true, None));

    (test_params, test_vectors_names)
}

fn fund_utxos(
    alice_wallet: Arc<SimpleWallet<&BitcoinCoreProvider, Arc<MemoryStorage>>>,
    bob_wallet: Arc<SimpleWallet<&BitcoinCoreProvider, Arc<MemoryStorage>>>,
    miner_client: &Client,
) {
    // Generate new fund addresses for Alice and Bob
    let alice_fund_address = alice_wallet.get_new_address().unwrap();
    let _ = alice_wallet
        .import_private_key_for_address(&alice_fund_address)
        .expect("Error: alice_wallet.import_address");
    let bob_fund_address = bob_wallet.get_new_address().unwrap();
    let _ = bob_wallet
        .import_private_key_for_address(&bob_fund_address)
        .expect("Error: bob_wallet.import_address");
    // Fund Alice and Bob's wallets (necessary to generate UTXOs before creating DLC contract)
    miner_client
        .send_to_address(
            &alice_fund_address,
            Amount::from_sat(100000000),
            None,
            None,
            Some(false),
            None,
            None,
            None,
        )
        .expect("Error: miner_client.send_to_address");
    miner_client
        .send_to_address(
            &bob_fund_address,
            Amount::from_sat(100000000),
            None,
            None,
            Some(false),
            None,
            None,
            None,
        )
        .expect("Error: miner_client.send_to_address");
    // Generate new miner address
    let miner_address = miner_client
        .get_new_address(None, Some(AddressType::Bech32))
        .unwrap();
    // Mine 1 new block to confirm previous transactions
    miner_client
        .generate_to_address(1, &miner_address)
        .expect("Error: miner_client.generate_to_address");
    // Refresh Alice and Bob's wallets balances in storage
    let _ = SimpleWallet::refresh(&alice_wallet).expect("Error: SimpleWallet::refresh");
    let _ = SimpleWallet::refresh(&bob_wallet).expect("Error: SimpleWallet::refresh");
}

fn generate_test_vector(
    rpc_provider: Arc<BitcoinCoreProvider>,
    alice_wallet: Arc<SimpleWallet<&BitcoinCoreProvider, Arc<MemoryStorage>>>,
    bob_wallet: Arc<SimpleWallet<&BitcoinCoreProvider, Arc<MemoryStorage>>>,
    alice_store: Arc<MemoryStorage>,
    bob_store: Arc<MemoryStorage>,
    test_params: TestParams,
    test_vector_name: &str,
) {
    println!("\n{}{}\n", "Generating test vector: ".bold().white(), test_vector_name);
    // Instantiate DLC managers for Alice and Bob
    let mut alice_oracles = HashMap::with_capacity(1);
    let mut bob_oracles = HashMap::with_capacity(1);
    let oracles = test_params.oracles;
    for oracle in oracles.into_iter() {
        let pub_key = oracle.get_public_key();
        let arc_oracle = Arc::new(oracle);
        alice_oracles.insert(pub_key, Arc::clone(&arc_oracle));
        bob_oracles.insert(pub_key, Arc::clone(&arc_oracle));
    }
    let mock_time = Arc::new(mocks::mock_time::MockTime {});
    mocks::mock_time::set_time((EVENT_MATURITY as u64) - 1);
    // let blockchain = Arc::new(rpc_provider);
    let mut alice_manager = Manager::new(
        alice_wallet,
        Arc::clone(&rpc_provider),
        alice_store,
        alice_oracles,
        Arc::clone(&mock_time),
        Arc::clone(&rpc_provider),
    )
        .unwrap();
    let mut bob_manager = Manager::new(
        bob_wallet,
        Arc::clone(&rpc_provider),
        bob_store,
        bob_oracles,
        Arc::clone(&mock_time),
        Arc::clone(&rpc_provider),
    )
        .unwrap();
    // Alice creates DLC offer message
    let offer_msg = alice_manager
        .send_offer(
            &test_params.contract_input,
            "0218845781f631c48f1c9709e23092067d06837f30aa0cd0544ac887fe91ddd166" // The public key of the counter-party's node.
                .parse()
                .unwrap(),
        )
        .expect("Send offer error");
    let temporary_contract_id = offer_msg.temporary_contract_id;
    // Bob verifies DLC offer message
    let _ = Manager::on_dlc_message(
        & mut bob_manager,
        &Message::Offer(offer_msg.clone()),
        "0218845781f631c48f1c9709e23092067d06837f30aa0cd0544ac887fe91ddd166" // The public key of the counter-party's node.
            .parse()
            .unwrap(),
    )
        .unwrap();
    // Bob accepts DLC offer message
    let (_, _, accept_msg) = bob_manager
        .accept_contract_offer(&temporary_contract_id)
        .unwrap();
    // Alice verifies DLC accept message and signs contract
    let sign_msg = match Manager::on_dlc_message(
        & mut alice_manager,
        &Message::Accept(accept_msg.clone()),
        "0218845781f631c48f1c9709e23092067d06837f30aa0cd0544ac887fe91ddd166" // The public key of the counter-party's node.
            .parse()
            .unwrap(),
    )
        .unwrap()
        .unwrap()
    {
        Message::Sign(sign_msg) => Ok(sign_msg),
        _ => Err("Error signing DLC message"),
    }
        .unwrap();
    // Bob verifies DLC sign message
    let _ = Manager::on_dlc_message(
        & mut bob_manager,
        &Message::Sign(sign_msg.clone()),
        "0218845781f631c48f1c9709e23092067d06837f30aa0cd0544ac887fe91ddd166" // The public key of the counter-party's node.
            .parse()
            .unwrap(),
    )
        .unwrap();
    // Save DLC test vector
    save_dlc_test_vector(test_vector_name, offer_msg, accept_msg, sign_msg);
}

fn init_clients(host: &str, usr: &str, pwd: &str) -> (Client, Client, Client, Client) {
    let auth = Auth::UserPass(usr.to_string(), pwd.to_string());
    // Instantiate RPC client
    let rpc_client = Client::new(host, auth.clone()).unwrap();
    // Generate client wallet instances
    let offer_client = get_new_wallet_rpc(&rpc_client, "Alice", auth.clone()).unwrap();
    let accept_client = get_new_wallet_rpc(&rpc_client, "Bob", auth.clone()).unwrap();
    let miner_client = get_new_wallet_rpc(&rpc_client, "Miner", auth.clone()).unwrap();
    // Generate new miner address
    let miner_address = miner_client
        .get_new_address(None, Some(AddressType::Bech32))
        .unwrap();
    // Mine new blocks to fund miner wallet
    miner_client
        .generate_to_address(110, &miner_address)
        .expect("Error: miner_client.generate_to_address");
    (offer_client, accept_client, miner_client, rpc_client)
}

fn init_wallets<'a>(
    offer_wallet_provider: &'a BitcoinCoreProvider,
    accept_wallet_provider: &'a BitcoinCoreProvider,
) -> (
    Arc<SimpleWallet<&'a BitcoinCoreProvider, Arc<MemoryStorage>>>,
    Arc<SimpleWallet<&'a BitcoinCoreProvider, Arc<MemoryStorage>>>,
    Arc<MemoryStorage>,
    Arc<MemoryStorage>,
) {
    // Instantiate SimpleWallet wallets for DLC parties Alice and Bob
    let alice_store = Arc::new(mocks::memory_storage_provider::MemoryStorage::new());
    let alice_wallet = Arc::new(SimpleWallet::new(
        offer_wallet_provider,
        alice_store.clone(),
        Network::Regtest,
    ));
    let bob_store = Arc::new(mocks::memory_storage_provider::MemoryStorage::new());
    let bob_wallet = Arc::new(SimpleWallet::new(
        accept_wallet_provider,
        bob_store.clone(),
        Network::Regtest,
    ));

    (alice_wallet, bob_wallet, alice_store, bob_store)
}

fn save_dlc_test_vector(
    file_name: &str,
    offer_msg: OfferDlc,
    accept_msg: AcceptDlc,
    sign_msg: SignDlc,
) {
    // Serialize the offer message
    let mut serialized_offer_msg_bytes = Vec::new();
    OFFER_TYPE
        .write(&mut serialized_offer_msg_bytes)
        .expect("Error writing offer_message type");
    // Then write the message itself
    offer_msg
        .write(&mut serialized_offer_msg_bytes)
        .expect("Error writing message");
    // Serialize the accept message
    let mut serialized_accept_msg_bytes = Vec::new();
    ACCEPT_TYPE
        .write(&mut serialized_accept_msg_bytes)
        .expect("Error writing accept_message type");
    // Then write the message itself
    accept_msg
        .write(&mut serialized_accept_msg_bytes)
        .expect("Error writing message");
    // Serialize the sign message
    let mut serialized_sign_msg_bytes = Vec::new();
    SIGN_TYPE
        .write(&mut serialized_sign_msg_bytes)
        .expect("Error writing sign_message type");
    // Then write the message itself
    sign_msg
        .write(&mut serialized_sign_msg_bytes)
        .expect("Error writing message");

    let offer_dlc_msg = DlcOfferMessage {
        message: offer_msg,
        serialized: hex::encode(&serialized_offer_msg_bytes),
    };
    let accept_dlc_msg = DlcAcceptMessage {
        message: accept_msg,
        serialized: hex::encode(&serialized_accept_msg_bytes),
    };
    let sign_dlc_msg = DlcSignMessage {
        message: sign_msg,
        serialized: hex::encode(&serialized_sign_msg_bytes),
    };
    let dlc_test_vector = DlcTestVector {
        offer_message: offer_dlc_msg,
        accept_message: accept_dlc_msg,
        sign_message: sign_dlc_msg,
    };
    let dlc_test_vector_string = serde_json::to_string_pretty(&dlc_test_vector).unwrap();
    // Write DLC test vector to file
    let path = "./test_vectors/".to_owned() + file_name + ".json";
    let _ = fs::write(&path, dlc_test_vector_string);
    println!(
        "\n{}{}\n",
        "DLC test vector saved to: ".bold().yellow(),
        path
    );
}
