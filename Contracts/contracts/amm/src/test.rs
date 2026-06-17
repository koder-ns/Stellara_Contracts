#![cfg(test)]

use shared::circuit_breaker::CircuitBreakerConfig;
use shared::governance::ProposalStatus;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::StellarAssetClient,
    vec, Address, Env,
};
use crate::{AmmContract, AmmContractClient};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn default_cb() -> CircuitBreakerConfig {
    CircuitBreakerConfig {
        max_volume_per_period: 1_000_000_000i128,
        max_tx_count_per_period: 1000u64,
        period_duration: 3600u64,
    }
}

fn setup(env: &Env) -> (AmmContractClient<'_>, Address, Address, Address) {
    let id = env.register_contract(None, AmmContract);
    let client = AmmContractClient::new(env, &id);
    let admin = Address::generate(env);
    let approver = Address::generate(env);
    let executor = Address::generate(env);
    env.mock_all_auths();
    client.init(&admin, &vec![env, approver.clone()], &executor, &default_cb());
    (client, admin, approver, executor)
}

/// Create real SAC token contracts; return (token_a, token_b).
fn make_tokens(env: &Env, admin: &Address) -> (Address, Address) {
    let ta = env.register_stellar_asset_contract(admin.clone());
    let tb = env.register_stellar_asset_contract(admin.clone());
    // Enforce canonical ordering so pool key is consistent
    if ta < tb { (ta, tb) } else { (tb, ta) }
}

/// Mint `amount` of both tokens to `recipient`.
fn mint(env: &Env, ta: &Address, tb: &Address, recipient: &Address, amount: i128) {
    StellarAssetClient::new(env, ta).mint(recipient, &amount);
    StellarAssetClient::new(env, tb).mint(recipient, &amount);
}

/// Create a pool and return (token_a, token_b).
fn make_pool(env: &Env, client: &AmmContractClient<'_>, admin: &Address) -> (Address, Address) {
    let (ta, tb) = make_tokens(env, admin);
    let init_sqrt: i128 = 1i128 << 64;
    client.create_pool(admin, &ta, &tb, &30u32, &init_sqrt);
    (ta, tb)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_init_success() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let (_client, _, _, _) = setup(&env);
}

#[test]
fn test_double_init_fails() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, approver, executor) = setup(&env);
    let result = client.try_init(&admin, &vec![&env, approver], &executor, &default_cb());
    assert!(result.is_err());
}

#[test]
fn test_create_pool_success() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let pool = client.get_pool(&ta, &tb).unwrap();
    assert_eq!(pool.fee_tier, 30);
    assert_eq!(pool.dynamic_fee_bps, 30);
    assert_eq!(pool.liquidity, 0);
}

#[test]
fn test_create_pool_duplicate_fails() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let result = client.try_create_pool(&admin, &ta, &tb, &30u32, &(1i128 << 64));
    assert!(result.is_err());
}

#[test]
fn test_invalid_fee_tier_fails() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_tokens(&env, &admin);
    let result = client.try_create_pool(&admin, &ta, &tb, &99u32, &(1i128 << 64));
    assert!(result.is_err());
}

#[test]
fn test_add_liquidity_in_range() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let lp = Address::generate(&env);
    mint(&env, &ta, &tb, &lp, 10_000_000);

    let r = client.add_liquidity(
        &lp, &ta, &tb, &-512i32, &512i32,
        &1_000_000i128, &1_000_000i128, &0i128, &0i128,
    );
    assert!(r.liquidity > 0);
    assert!(r.position_id >= 1);
}

#[test]
fn test_invalid_tick_range_fails() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let lp = Address::generate(&env);
    let result = client.try_add_liquidity(
        &lp, &ta, &tb, &512i32, &-512i32,
        &1_000_000i128, &1_000_000i128, &0i128, &0i128,
    );
    assert!(result.is_err());
}

#[test]
fn test_get_position_after_add() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let lp = Address::generate(&env);
    mint(&env, &ta, &tb, &lp, 10_000_000);

    let r = client.add_liquidity(
        &lp, &ta, &tb, &-512i32, &512i32,
        &1_000_000i128, &1_000_000i128, &0i128, &0i128,
    );
    let pos = client.get_position(&r.position_id).unwrap();
    assert_eq!(pos.tick_lower, -512);
    assert_eq!(pos.tick_upper, 512);
    assert_eq!(pos.owner, lp);
}

#[test]
fn test_get_lp_positions_by_owner() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let lp = Address::generate(&env);
    mint(&env, &ta, &tb, &lp, 10_000_000);

    client.add_liquidity(&lp, &ta, &tb, &-512i32, &512i32, &500_000i128, &500_000i128, &0i128, &0i128);
    client.add_liquidity(&lp, &ta, &tb, &-1024i32, &1024i32, &500_000i128, &500_000i128, &0i128, &0i128);

    assert_eq!(client.get_lp_positions(&lp).len(), 2);
}

#[test]
fn test_remove_liquidity_full() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let lp = Address::generate(&env);
    mint(&env, &ta, &tb, &lp, 10_000_000);

    let r = client.add_liquidity(&lp, &ta, &tb, &-512i32, &512i32, &1_000_000i128, &1_000_000i128, &0i128, &0i128);
    client.remove_liquidity(&lp, &r.position_id, &r.liquidity, &0i128, &0i128);
    assert!(client.get_position(&r.position_id).is_none());
}

#[test]
fn test_remove_liquidity_partial() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let lp = Address::generate(&env);
    mint(&env, &ta, &tb, &lp, 10_000_000);

    let r = client.add_liquidity(&lp, &ta, &tb, &-512i32, &512i32, &1_000_000i128, &1_000_000i128, &0i128, &0i128);
    let half = r.liquidity / 2;
    client.remove_liquidity(&lp, &r.position_id, &half, &0i128, &0i128);
    let pos = client.get_position(&r.position_id).unwrap();
    assert_eq!(pos.liquidity, r.liquidity - half);
}

#[test]
fn test_remove_liquidity_wrong_owner_fails() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let lp = Address::generate(&env);
    let intruder = Address::generate(&env);
    mint(&env, &ta, &tb, &lp, 10_000_000);

    let r = client.add_liquidity(&lp, &ta, &tb, &-512i32, &512i32, &1_000_000i128, &1_000_000i128, &0i128, &0i128);
    let result = client.try_remove_liquidity(&intruder, &r.position_id, &r.liquidity, &0i128, &0i128);
    assert!(result.is_err());
}

#[test]
fn test_estimate_il_at_entry_near_zero() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let lp = Address::generate(&env);
    mint(&env, &ta, &tb, &lp, 10_000_000);

    let r = client.add_liquidity(&lp, &ta, &tb, &-512i32, &512i32, &1_000_000i128, &1_000_000i128, &0i128, &0i128);
    let il = client.estimate_il(&r.position_id);
    assert!(il.abs() < 100, "IL at entry should be < 100 bps, got {}", il);
}

#[test]
fn test_capital_efficiency_returns_valid_range() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let lp = Address::generate(&env);
    mint(&env, &ta, &tb, &lp, 10_000_000);

    client.add_liquidity(&lp, &ta, &tb, &-512i32, &512i32, &1_000_000i128, &1_000_000i128, &0i128, &0i128);
    let eff = client.get_capital_efficiency(&ta, &tb);
    assert!(eff >= 0 && eff <= 10_000);
}

#[test]
fn test_update_dynamic_fee_no_history_stays_base() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    assert_eq!(client.update_dynamic_fee(&ta, &tb), 30);
}

#[test]
fn test_pool_not_found_errors() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_tokens(&env, &admin);
    assert!(client.try_update_dynamic_fee(&ta, &tb).is_err());
    assert!(client.try_get_capital_efficiency(&ta, &tb).is_err());
}

#[test]
fn test_position_not_found_error() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    let (client, _, _, _) = setup(&env);
    assert!(client.try_estimate_il(&9999u64).is_err());
}

#[test]
fn test_pause_blocks_add_liquidity() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    client.pause(&admin);
    let lp = Address::generate(&env);
    let result = client.try_add_liquidity(
        &lp, &ta, &tb, &-512i32, &512i32,
        &1_000_000i128, &1_000_000i128, &0i128, &0i128,
    );
    assert!(result.is_err());
}

#[test]
fn test_unpause_restores_operations() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    client.pause(&admin);
    client.unpause(&admin);
    let lp = Address::generate(&env);
    mint(&env, &ta, &tb, &lp, 10_000_000);
    let result = client.try_add_liquidity(
        &lp, &ta, &tb, &-512i32, &512i32,
        &1_000_000i128, &1_000_000i128, &0i128, &0i128,
    );
    assert!(result.is_ok());
}

#[test]
fn test_multiple_positions_same_pool() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let (ta, tb) = make_pool(&env, &client, &admin);
    let lp1 = Address::generate(&env);
    let lp2 = Address::generate(&env);
    mint(&env, &ta, &tb, &lp1, 10_000_000);
    mint(&env, &ta, &tb, &lp2, 10_000_000);

    let r1 = client.add_liquidity(&lp1, &ta, &tb, &-512i32, &512i32, &500_000i128, &500_000i128, &0i128, &0i128);
    let r2 = client.add_liquidity(&lp2, &ta, &tb, &-1024i32, &1024i32, &500_000i128, &500_000i128, &0i128, &0i128);

    assert_ne!(r1.position_id, r2.position_id);
    assert_eq!(client.get_position(&r1.position_id).unwrap().tick_lower, -512);
    assert_eq!(client.get_position(&r2.position_id).unwrap().tick_lower, -1024);
}

#[test]
fn test_governance_upgrade_flow() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, approver, executor) = setup(&env);

    let pid = client.propose_upgrade(
        &admin,
        &soroban_sdk::symbol_short!("v2hash"),
        &soroban_sdk::symbol_short!("Upgrade"),
        &vec![&env, approver.clone()],
        &1u32,
        &3600u64,
    );
    client.approve_upgrade(&pid, &approver);
    assert_eq!(client.get_upgrade_proposal(&pid).status, ProposalStatus::Approved);

    env.ledger().with_mut(|l| l.timestamp = 1000 + 3601);
    client.execute_upgrade(&pid, &executor);
    assert_eq!(client.get_upgrade_proposal(&pid).status, ProposalStatus::Executed);
}

#[test]
fn test_all_three_fee_tiers_create_pools() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.timestamp = 1000);
    env.mock_all_auths();
    let (client, admin, _, _) = setup(&env);
    let init_sqrt: i128 = 1i128 << 64;

    for tier in [5u32, 30u32, 100u32] {
        let (ta, tb) = make_tokens(&env, &admin);
        client.create_pool(&admin, &ta, &tb, &tier, &init_sqrt);
        let pool = client.get_pool(&ta, &tb).unwrap();
        assert_eq!(pool.fee_tier, tier);
        assert_eq!(pool.dynamic_fee_bps, tier);
    }
}
