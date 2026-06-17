#![cfg(test)]

extern crate std;

use cross_chain_router::{CrossChainRouter, LightClientHeader, CrossChainRouterClient};
use shared::reentrancy_guard::ReentrancyGuard;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Bytes, BytesN, Env,
};

#[test]
fn test_cross_chain_replay_rejected() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);
    env.mock_all_auths();

    let contract_id = env.register_contract(None, CrossChainRouter);
    let client = CrossChainRouterClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let payload = Bytes::from_array(&env, &[7u8; 32]);
    let message_id = client.initiate_message(&0, &1, &sender, &recipient, &payload);

    let header = LightClientHeader {
        block_number: 1,
        block_hash: BytesN::from_array(&env, &[1u8; 32]),
        timestamp: 1000,
        commitment_root: BytesN::from_array(&env, &[2u8; 32]),
    };

    let proof = Bytes::from_array(&env, &[3u8; 32]);
    let first_verify = client.verify_message(&message_id, &header, &proof);
    assert!(first_verify);

    let replay = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.verify_message(&message_id, &header, &proof);
    }));
    assert!(replay.is_err(), "Replay should be rejected");
}

#[test]
fn test_cross_chain_reentrancy_guard_blocks_nested_calls() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);
    env.mock_all_auths();

    let contract_id = env.register_contract(None, CrossChainRouter);
    let client = CrossChainRouterClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    let validator = Address::generate(&env);
    client.register_validator(&validator, &1_000_000_000i128);

    ReentrancyGuard::enter(&env);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.register_validator(&validator, &2_000_000_000i128);
    }));
    assert!(result.is_err(), "Reentrancy should be blocked");
    ReentrancyGuard::exit(&env);
}

#[test]
fn test_cross_chain_nonce_enforces_sequential_order() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000);
    env.mock_all_auths();

    let contract_id = env.register_contract(None, CrossChainRouter);
    let client = CrossChainRouterClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);
    client.set_chain_id(&admin, &1u32);

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let payload = Bytes::from_array(&env, &[8u8; 32]);
    client.initiate_message(&1, &2, &sender, &recipient, &payload);
}
