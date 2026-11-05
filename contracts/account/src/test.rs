use crate::fixture::*;
use crate::{authorize, Error, SemaphoreAccount, SemaphoreAccountClient, SemaphoreSig, VkData};
use soroban_sdk::{vec, Address, BytesN, Env};

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

/// BN254 scalar field order `r`, big-endian.
const R_BE: [u8; 32] = [
    0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58, 0x5d,
    0x28, 0x33, 0xe8, 0x48, 0x79, 0xb9, 0x70, 0x91, 0x43, 0xe1, 0xf5, 0x93, 0xf0, 0x00, 0x00, 0x01,
];

fn add_be(x: &[u8; 32], y: &[u8; 32]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut carry = 0u16;
    for i in (0..32).rev() {
        let s = x[i] as u16 + y[i] as u16 + carry;
        out[i] = (s & 0xff) as u8;
        carry = s >> 8;
    }
    out
}

/// Replay hardening: after a legitimate authorization with nullifier `n`, re-presenting the SAME
/// proof with the nullifier re-encoded as `n + r` (same field element, different bytes) must be
/// rejected as non-canonical — otherwise the distinct storage key would evade the replay guard.
#[test]
fn rejects_non_canonical_nullifier_replay() {
    let env = Env::default();
    let id = setup(&env);
    let payload = BytesN::from_array(&env, &PAYLOAD);
    let first = env.as_contract(&id, || authorize(&env, &payload, &sig(&env)));
    assert_eq!(first, Ok(()));
    // Same proof, nullifier = n + r.
    let nplus_r = add_be(&NULLIFIER, &R_BE);
    let mut bad = sig(&env);
    bad.nullifier = BytesN::from_array(&env, &nplus_r);
    let second = env.as_contract(&id, || authorize(&env, &payload, &bad));
    assert_eq!(second, Err(Error::NonCanonicalInput));
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
