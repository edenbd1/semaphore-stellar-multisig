use crate::fixture::*;
use crate::{ConfidentialDao, ConfidentialDaoClient, Error, ProofEntry, VkData};
use soroban_sdk::{vec, Address, BytesN, Env};

fn vk(env: &Env) -> VkData {
    VkData {
        alpha: BytesN::from_array(env, &VK_ALPHA),
        beta: BytesN::from_array(env, &VK_BETA),
        gamma: BytesN::from_array(env, &VK_GAMMA),
        delta: BytesN::from_array(env, &VK_DELTA),
        ic: vec![
            env,
            BytesN::from_array(env, &VK_IC_0),
            BytesN::from_array(env, &VK_IC_1),
            BytesN::from_array(env, &VK_IC_2),
            BytesN::from_array(env, &VK_IC_3),
            BytesN::from_array(env, &VK_IC_4),
        ],
    }
}

fn deploy(env: &Env, threshold: u32) -> Address {
    // Register at the FIXED address the fixture's domain-bound proposal scope was proven
    // against, so on-chain `current_contract_address()` matches what the prover folded in.
    env.register_at(
        &Address::from_str(env, CONTRACT),
        ConfidentialDao,
        (BytesN::from_array(env, &ROOT), 2u32, threshold, vk(env)),
    )
}

fn entry_a(env: &Env) -> ProofEntry {
    ProofEntry {
        a: BytesN::from_array(env, &A_A),
        b: BytesN::from_array(env, &A_B),
        c: BytesN::from_array(env, &A_C),
        nullifier: BytesN::from_array(env, &NUL_A),
    }
}
fn entry_b(env: &Env) -> ProofEntry {
    ProofEntry {
        a: BytesN::from_array(env, &B_A),
        b: BytesN::from_array(env, &B_B),
        c: BytesN::from_array(env, &B_C),
        nullifier: BytesN::from_array(env, &NUL_B),
    }
}

// The fixture proofs bind message == hash(PAYLOAD action) and scope == domain(SCOPE_SEED), so a
// proposal is created with PAYLOAD as its action digest and SCOPE_SEED as its (domain-bound) seed.
fn payload(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &PAYLOAD)
}
fn scope_seed(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &SCOPE_SEED)
}

/// Two members approve one proof per transaction; at threshold the proposal executes.
#[test]
fn approvals_accumulate_then_execute() {
    let env = Env::default();
    let client = ConfidentialDaoClient::new(&env, &deploy(&env, 2));
    let p = payload(&env);
    let id = client.create_proposal(&p, &scope_seed(&env));

    assert_eq!(client.approve(&id, &entry_a(&env)), 1);
    assert_eq!(client.approve(&id, &entry_b(&env)), 2);
    client.execute(&id);
    assert!(client.proposal(&id).executed);
}

/// A member cannot approve the same proposal twice (nullifier is per-proposal).
#[test]
fn rejects_double_approval() {
    let env = Env::default();
    let client = ConfidentialDaoClient::new(&env, &deploy(&env, 2));
    let p = payload(&env);
    let id = client.create_proposal(&p, &scope_seed(&env));
    client.approve(&id, &entry_a(&env));
    let again = client.try_approve(&id, &entry_a(&env));
    assert_eq!(again, Err(Ok(Error::AlreadyApproved)));
}

/// Execution before the threshold is rejected.
#[test]
fn rejects_execute_below_threshold() {
    let env = Env::default();
    let client = ConfidentialDaoClient::new(&env, &deploy(&env, 2));
    let p = payload(&env);
    let id = client.create_proposal(&p, &scope_seed(&env));
    client.approve(&id, &entry_a(&env)); // only 1 of 2
    let r = client.try_execute(&id);
    assert_eq!(r, Err(Ok(Error::ThresholdNotMet)));
}

/// Two proposals cannot share a scope_seed: that is what makes an approval bind to one specific
/// proposal. Without it, approvals cast for proposal A could be replayed on a duplicate B.
#[test]
fn rejects_reused_scope_seed() {
    let env = Env::default();
    let client = ConfidentialDaoClient::new(&env, &deploy(&env, 2));
    let p = payload(&env);
    let _id = client.create_proposal(&p, &scope_seed(&env));
    // Reusing the same scope_seed (even for a different action) must be rejected.
    let mut other = PAYLOAD;
    other[0] ^= 0xff;
    let again = client.try_create_proposal(&BytesN::from_array(&env, &other), &scope_seed(&env));
    assert_eq!(again, Err(Ok(Error::ScopeSeedReused)));
}

/// An approval whose proof is not bound to the proposal's action is rejected.
#[test]
fn rejects_approval_for_wrong_action() {
    let env = Env::default();
    let client = ConfidentialDaoClient::new(&env, &deploy(&env, 2));
    let mut other = PAYLOAD;
    other[0] ^= 0xff;
    // scope_seed stays SCOPE_SEED (so nullifier/scope match), but action differs -> message mismatch.
    let id = client.create_proposal(&BytesN::from_array(&env, &other), &scope_seed(&env));
    let r = client.try_approve(&id, &entry_a(&env));
    assert_eq!(r, Err(Ok(Error::InvalidProof)));
}

/// Domain separation: an approval proof for a proposal in THIS DAO is rejected by another DAO
/// at a different address (identical group root, same action and scope seed). The on-chain scope
/// is bound to the DAO's own address, so approvals cannot be replayed across DAOs.
#[test]
fn rejects_cross_contract_replay() {
    let env = Env::default();
    // A second DAO at a DIFFERENT address, same config.
    let other = env.register(
        ConfidentialDao,
        (BytesN::from_array(&env, &ROOT), 2u32, 2u32, vk(&env)),
    );
    let client = ConfidentialDaoClient::new(&env, &other);
    let id = client.create_proposal(&payload(&env), &scope_seed(&env));
    let r = client.try_approve(&id, &entry_a(&env));
    assert_eq!(r, Err(Ok(Error::InvalidProof)));
}
