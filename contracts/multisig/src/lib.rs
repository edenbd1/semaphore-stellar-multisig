#![no_std]
//! Phase-3: a **confidential threshold multisig** custom account.
//!
//! The account is backed by a Semaphore group of N members and a threshold `k`. To authorize
//! a transaction, `k` *distinct* members each submit a Semaphore proof. `__check_auth`:
//!   1. binds every proof's signal **and** scope to `signature_payload`
//!      (`message = scope = keccak256(payload) >> 8`), so the proofs authorize exactly this
//!      transaction — not another, and not a weaker rule (anti rule-downgrade);
//!   2. verifies each proof against the group root + verification key;
//!   3. requires the proofs' nullifiers to be **pairwise distinct** — since a nullifier is
//!      deterministic in `(scope, member)`, distinct nullifiers for one payload prove `k`
//!      distinct members signed, *without revealing which ones*;
//!   4. requires `k >= threshold`.
//!
//! Replay across transactions is impossible (scope is payload-derived) and replay of the same
//! transaction is prevented by the auth entry's own nonce in `signature_payload`, so no
//! persistent nullifier registry is needed here.

use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
    Bytes, BytesN, Env, Map, Vec,
};
use semaphore_verifier::verify_encoded;

/// BN254 scalar field order `r`, big-endian (canonical-input guard).
const R_BE: [u8; 32] = [
    0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58, 0x5d,
    0x28, 0x33, 0xe8, 0x48, 0x79, 0xb9, 0x70, 0x91, 0x43, 0xe1, 0xf5, 0x93, 0xf0, 0x00, 0x00, 0x01,
];
fn is_canonical(v: &BytesN<32>) -> bool {
    v.to_array() < R_BE
}

/// Semaphore's field hash of a 32-byte value: `keccak256(value) >> 8`.
fn semaphore_hash(env: &Env, value: &BytesN<32>) -> BytesN<32> {
    let k = env.crypto().keccak256(&Bytes::from(value.clone())).to_array();
    let mut out = [0u8; 32];
    out[1..32].copy_from_slice(&k[0..31]);
    BytesN::from_array(env, &out)
}

/// Domain-separated scope: fold the multisig's OWN contract address into the scope so the k
/// proofs minted for a transaction here cannot be replayed against another contract that shares
/// the group root. The prover binds `scope` to `keccak256( ascii(contract_strkey) || payload )`
/// (then `>> 8`); we recompute it from `env.current_contract_address()`, unforgeable by others.
fn domain_scope_hash(env: &Env, scope_seed: &BytesN<32>) -> BytesN<32> {
    let strkey = env.current_contract_address().to_string();
    let mut preimage = strkey.to_bytes();
    preimage.append(&Bytes::from(scope_seed.clone()));
    let rawscope = BytesN::from_array(env, &env.crypto().keccak256(&preimage).to_array());
    semaphore_hash(env, &rawscope)
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    ThresholdNotMet = 3,
    DuplicateNullifier = 4,
    NonCanonicalInput = 5,
    InvalidProof = 6,
}

#[contracttype]
#[derive(Clone)]
pub struct VkData {
    pub alpha: BytesN<64>,
    pub beta: BytesN<128>,
    pub gamma: BytesN<128>,
    pub delta: BytesN<128>,
    pub ic: Vec<BytesN<64>>,
}

/// One member's Semaphore proof entry.
#[contracttype]
#[derive(Clone)]
pub struct ProofEntry {
    pub a: BytesN<64>,
    pub b: BytesN<128>,
    pub c: BytesN<64>,
    pub nullifier: BytesN<32>,
}

#[contracttype]
#[derive(Clone)]
pub struct Config {
    pub root: BytesN<32>,
    pub depth: u32,
    pub threshold: u32,
    pub vk: VkData,
}

#[contracttype]
pub enum DataKey {
    Config,
}

/// Core threshold-authorization logic, parameterised on the raw 32-byte payload so it is
/// unit-testable without a `Hash<32>`. Reads config from the account's storage.
pub fn authorize(env: &Env, payload: &BytesN<32>, proofs: &Vec<ProofEntry>) -> Result<(), Error> {
    let cfg: Config = env
        .storage()
        .instance()
        .get(&DataKey::Config)
        .ok_or(Error::NotInitialized)?;

    if proofs.len() < cfg.threshold {
        return Err(Error::ThresholdNotMet);
    }

    // message := Semaphore-hash(payload) binds every proof to this transaction; scope is
    // domain-bound to THIS contract, so the proofs cannot be replayed against another multisig.
    let message = semaphore_hash(env, payload);
    let scope = domain_scope_hash(env, payload);

    let mut seen: Map<BytesN<32>, ()> = Map::new(env);
    for e in proofs.iter() {
        if !is_canonical(&e.nullifier) {
            return Err(Error::NonCanonicalInput);
        }
        if seen.contains_key(e.nullifier.clone()) {
            return Err(Error::DuplicateNullifier);
        }
        seen.set(e.nullifier.clone(), ());

        let pub_signals = soroban_sdk::vec![
            env,
            cfg.root.clone(),
            e.nullifier.clone(),
            message.clone(),
            scope.clone()
        ];
        let ok = verify_encoded(
            env,
            &pub_signals,
            &e.a,
            &e.b,
            &e.c,
            &cfg.vk.alpha,
            &cfg.vk.beta,
            &cfg.vk.gamma,
            &cfg.vk.delta,
            &cfg.vk.ic,
        );
        if !ok {
            return Err(Error::InvalidProof);
        }
    }
    Ok(())
}

#[contract]
pub struct SemaphoreMultisig;

#[contractimpl]
impl SemaphoreMultisig {
    /// Configure the group root, tree depth, threshold and verification key (once).
    pub fn __constructor(env: Env, root: BytesN<32>, depth: u32, threshold: u32, vk: VkData) {
        env.storage()
            .instance()
            .set(&DataKey::Config, &Config { root, depth, threshold, vk });
    }

    /// Current threshold (read helper).
    pub fn threshold(env: Env) -> Result<u32, Error> {
        let cfg: Config = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;
        Ok(cfg.threshold)
    }

    /// A no-op that requires the account to authorize itself — invoking it externally triggers
    /// `__check_auth`, the entry point for the end-to-end on-chain threshold-auth demo.
    pub fn protected(env: Env) -> u32 {
        env.current_contract_address().require_auth();
        42
    }
}

#[contractimpl]
impl CustomAccountInterface for SemaphoreMultisig {
    type Signature = Vec<ProofEntry>;
    type Error = Error;

    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        signatures: Vec<ProofEntry>,
        _auth_contexts: Vec<Context>,
    ) -> Result<(), Error> {
        authorize(&env, &signature_payload.to_bytes(), &signatures)
    }
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod fixture;
#[cfg(test)]
mod test;
