use crate::fixture::*;
use crate::{authorize, Error, ProofEntry, SemaphoreMultisig, VkData};
use soroban_sdk::{vec, Address, BytesN, Env, Vec};

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

/// Register a 2-of-3 confidential multisig.
fn setup(env: &Env, threshold: u32) -> Address {
    env.register(
        SemaphoreMultisig,
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

/// Two distinct members meet the 2-of-3 threshold and authorize.
#[test]
fn authorizes_with_threshold_distinct_members() {
    let env = Env::default();
    let id = setup(&env, 2);
    let payload = BytesN::from_array(&env, &PAYLOAD);
    let proofs: Vec<ProofEntry> = vec![&env, entry_a(&env), entry_b(&env)];
    let r = env.as_contract(&id, || authorize(&env, &payload, &proofs));
    assert_eq!(r, Ok(()));
}

/// Fewer proofs than the threshold is rejected.
#[test]
fn rejects_below_threshold() {
    let env = Env::default();
    let id = setup(&env, 2);
    let payload = BytesN::from_array(&env, &PAYLOAD);
    let proofs: Vec<ProofEntry> = vec![&env, entry_a(&env)];
    let r = env.as_contract(&id, || authorize(&env, &payload, &proofs));
    assert_eq!(r, Err(Error::ThresholdNotMet));
}

/// The same member cannot be counted twice: a repeated nullifier is rejected.
#[test]
fn rejects_duplicate_member() {
    let env = Env::default();
    let id = setup(&env, 2);
    let payload = BytesN::from_array(&env, &PAYLOAD);
    let proofs: Vec<ProofEntry> = vec![&env, entry_a(&env), entry_a(&env)];
    let r = env.as_contract(&id, || authorize(&env, &payload, &proofs));
    assert_eq!(r, Err(Error::DuplicateNullifier));
}

/// Budget: report the CPU cost of a 2-of-n check. Each proof is one pairing check (~29M
/// instructions), so the achievable threshold per transaction is bounded by the 100M
/// per-tx limit (~3 proofs); larger thresholds need batching / accumulation (future work).
#[test]
fn reports_2of_n_budget() {
    let env = Env::default();
    let id = setup(&env, 2);
    let payload = BytesN::from_array(&env, &PAYLOAD);
    let proofs: Vec<ProofEntry> = vec![&env, entry_a(&env), entry_b(&env)];
    let r = env.as_contract(&id, || authorize(&env, &payload, &proofs));
    assert_eq!(r, Ok(()));
    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    std::println!("2-of-n authorize(): cpu_instructions = {cpu}");
    assert!(cpu < 100_000_000, "2-of-n must fit the mainnet per-tx CPU limit");
}

/// Proofs presented against a different payload (transaction) fail: the signal/scope binding
/// forces message == scope == hash(payload), so the proofs no longer verify.
#[test]
fn rejects_wrong_payload() {
    let env = Env::default();
    let id = setup(&env, 2);
    let mut bad = PAYLOAD;
    bad[0] ^= 0xff;
    let payload = BytesN::from_array(&env, &bad);
    let proofs: Vec<ProofEntry> = vec![&env, entry_a(&env), entry_b(&env)];
    let r = env.as_contract(&id, || authorize(&env, &payload, &proofs));
    assert_eq!(r, Err(Error::InvalidProof));
}
