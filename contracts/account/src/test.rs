use crate::fixture::*;
use crate::{authorize, Error, SemaphoreAccount, SemaphoreAccountClient, SemaphoreSig, VkData};
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env};

fn setup(env: &Env) -> Address {
    let id = env.register(SemaphoreAccount, ());
    let vk = VkData {
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
    };
    SemaphoreAccountClient::new(env, &id).init(
        &BytesN::from_array(env, &ROOT),
        &BytesN::from_array(env, &SCOPE_HASH),
        &vk,
    );
    id
}

fn sig(env: &Env) -> SemaphoreSig {
    SemaphoreSig {
        a: BytesN::from_array(env, &PROOF_A),
        b: BytesN::from_array(env, &PROOF_B),
        c: BytesN::from_array(env, &PROOF_C),
        nullifier: BytesN::from_array(env, &NULLIFIER),
    }
}

/// A valid Semaphore proof whose signal is bound to the payload authorizes the account.
#[test]
fn authorizes_with_valid_proof() {
    let env = Env::default();
    let id = setup(&env);
    let payload = BytesN::from_array(&env, &PAYLOAD);
    let sig = sig(&env);
    let r = env.as_contract(&id, || authorize(&env, &payload, &sig));
    assert_eq!(r, Ok(()));
}

/// The same nullifier cannot authorize twice (replay guard).
#[test]
fn rejects_nullifier_replay() {
    let env = Env::default();
    let id = setup(&env);
    let payload = BytesN::from_array(&env, &PAYLOAD);
    let sig = sig(&env);
    let first = env.as_contract(&id, || authorize(&env, &payload, &sig));
    assert_eq!(first, Ok(()));
    let second = env.as_contract(&id, || authorize(&env, &payload, &sig));
    assert_eq!(second, Err(Error::NullifierAlreadyUsed));
}

/// A proof presented against a different payload (transaction) fails: the signal binding
/// forces message := hash(payload), so the proof no longer verifies.
#[test]
fn rejects_wrong_payload() {
    let env = Env::default();
    let id = setup(&env);
    let mut bad = PAYLOAD;
    bad[0] ^= 0xff;
    let payload = BytesN::from_array(&env, &bad);
    let sig = sig(&env);
    let r = env.as_contract(&id, || authorize(&env, &payload, &sig));
    assert_eq!(r, Err(Error::InvalidProof));
}
