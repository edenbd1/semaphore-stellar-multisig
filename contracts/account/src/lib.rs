#![no_std]
//! Phase-1(c) prototype: a Soroban custom account whose authorization is an anonymous
//! Semaphore membership proof.
//!
//! `__check_auth` accepts a Semaphore proof as its signature and enforces:
//!   1. the proof verifies against the account's configured group root and verification key;
//!   2. the proof's **signal is bound to `signature_payload`** — the circuit's public
//!      `message` must be `keccak256(signature_payload) >> 8` (Semaphore's field hash), so a
//!      proof authorizes exactly this transaction and cannot be replayed on another;
//!   3. the proof's **nullifier has not been used before** (persistent registry), so each
//!      membership can authorize at most once per scope.
//!
//! The result: whoever controls a group member identity can authorize the account's actions
//! without revealing *which* member — anonymous, replay-protected authorization.

use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
    Bytes, BytesN, Env, Vec,
};
use semaphore_verifier::{is_canonical, verify_encoded};

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod fixture;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    InvalidProof = 3,
    NullifierAlreadyUsed = 4,
    NonCanonicalInput = 5,
}

/// Groth16/BN254 verification key, stored at construction.
#[contracttype]
#[derive(Clone)]
pub struct VkData {
    pub alpha: BytesN<64>,
    pub beta: BytesN<128>,
    pub gamma: BytesN<128>,
    pub delta: BytesN<128>,
    pub ic: Vec<BytesN<64>>,
}

/// A Semaphore proof used as an account signature.
#[contracttype]
#[derive(Clone)]
pub struct SemaphoreSig {
    pub a: BytesN<64>,
    pub b: BytesN<128>,
    pub c: BytesN<64>,
    /// The circuit's nullifier public output (double-authorization guard).
    pub nullifier: BytesN<32>,
}

#[contracttype]
pub enum DataKey {
    Root,
    ScopeHash,
    Vk,
    Nullifier(BytesN<32>),
}

/// Semaphore's field hash of a 32-byte value: `keccak256(value) >> 8`.
fn semaphore_hash(env: &Env, value: &BytesN<32>) -> BytesN<32> {
    let k = env.crypto().keccak256(&Bytes::from(value.clone())).to_array();
    let mut out = [0u8; 32];
    // right-shift the 256-bit big-endian value by 8 bits: drop the last byte.
    out[1..32].copy_from_slice(&k[0..31]);
    BytesN::from_array(env, &out)
}

/// Core authorization logic, parameterised on the raw 32-byte payload so it is unit-testable
/// without constructing a `Hash<32>` (whose constructor is SDK-internal). Must run inside the
/// account's contract context (it reads/writes the account's storage).
pub fn authorize(env: &Env, payload: &BytesN<32>, sig: &SemaphoreSig) -> Result<(), Error> {
    let store = env.storage().instance();
    let root: BytesN<32> = store.get(&DataKey::Root).ok_or(Error::NotInitialized)?;
    let scope_hash: BytesN<32> = store.get(&DataKey::ScopeHash).ok_or(Error::NotInitialized)?;
    let vk: VkData = store.get(&DataKey::Vk).ok_or(Error::NotInitialized)?;

    // Reject a non-canonical nullifier (>= r). The verifier reduces mod r, so without this a
    // member could resubmit the same proof with nullifier `n + r` (a distinct storage key that
    // still verifies) and bypass the replay guard below.
    if !is_canonical(&sig.nullifier) {
        return Err(Error::NonCanonicalInput);
    }

    // Bind the signal to this transaction: message := Semaphore-hash(payload).
    let message = semaphore_hash(env, payload);

    // Public signals (Semaphore v4 order): [root, nullifier, hash(message), hash(scope)].
    let pub_signals =
        soroban_sdk::vec![env, root, sig.nullifier.clone(), message, scope_hash];

    let ok = verify_encoded(
        env,
        &pub_signals,
        &sig.a,
        &sig.b,
        &sig.c,
        &vk.alpha,
        &vk.beta,
        &vk.gamma,
        &vk.delta,
        &vk.ic,
    );
    if !ok {
        return Err(Error::InvalidProof);
    }

    // Replay guard: a nullifier may authorize at most once (per scope).
    let key = DataKey::Nullifier(sig.nullifier.clone());
    if env.storage().persistent().has(&key) {
        return Err(Error::NullifierAlreadyUsed);
    }
    env.storage().persistent().set(&key, &());
    Ok(())
}

#[contract]
pub struct SemaphoreAccount;

#[contractimpl]
impl SemaphoreAccount {
    /// Configure the account's group root, scope hash and verification key (once).
    pub fn init(
        env: Env,
        root: BytesN<32>,
        scope_hash: BytesN<32>,
        vk: VkData,
    ) -> Result<(), Error> {
        let store = env.storage().instance();
        if store.has(&DataKey::Root) {
            return Err(Error::AlreadyInitialized);
        }
        store.set(&DataKey::Root, &root);
        store.set(&DataKey::ScopeHash, &scope_hash);
        store.set(&DataKey::Vk, &vk);
        Ok(())
    }

    /// A no-op that requires the account to authorize itself — invoking it externally triggers
    /// `__check_auth`, so it is the entry point for the end-to-end on-chain auth demo.
    pub fn protected(env: Env) -> u32 {
        env.current_contract_address().require_auth();
        42
    }
}

#[contractimpl]
impl CustomAccountInterface for SemaphoreAccount {
    type Signature = SemaphoreSig;
    type Error = Error;

    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        signatures: SemaphoreSig,
        _auth_contexts: Vec<Context>,
    ) -> Result<(), Error> {
        authorize(&env, &signature_payload.to_bytes(), &signatures)
    }
}

#[cfg(test)]
mod test;
