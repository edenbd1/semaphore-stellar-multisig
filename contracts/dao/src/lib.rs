#![no_std]
//! Phase-3 (cont.): a **proposal-based confidential multisig** that removes the per-transaction
//! threshold ceiling.
//!
//! Instead of submitting `k` proofs in one transaction (bounded to ~3 by the 100M instruction
//! limit), members approve a proposal **asynchronously, one proof per transaction**. The
//! contract accumulates distinct nullifiers until the threshold is reached, then the proposal
//! can execute. This is the usual DAO flow (create → approve → execute), made anonymous: you
//! never learn which members approved, only that `k` distinct ones did.
//!
//! Binding: each approval proof must carry `message = hash(action)` and `scope = hash(scope_seed)`
//! where `scope_seed` is unique per proposal. So an approval authorizes a specific action on a
//! specific proposal, and a member's nullifier is unique per proposal (approve-once).

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Bytes, BytesN, Env, Vec,
};
use semaphore_verifier::verify_encoded;

const R_BE: [u8; 32] = [
    0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58, 0x5d,
    0x28, 0x33, 0xe8, 0x48, 0x79, 0xb9, 0x70, 0x91, 0x43, 0xe1, 0xf5, 0x93, 0xf0, 0x00, 0x00, 0x01,
];
fn is_canonical(v: &BytesN<32>) -> bool {
    v.to_array() < R_BE
}
fn semaphore_hash(env: &Env, value: &BytesN<32>) -> BytesN<32> {
    let k = env.crypto().keccak256(&Bytes::from(value.clone())).to_array();
    let mut out = [0u8; 32];
    out[1..32].copy_from_slice(&k[0..31]);
    BytesN::from_array(env, &out)
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    ProposalNotFound = 2,
    AlreadyExecuted = 3,
    AlreadyApproved = 4,
    NonCanonicalInput = 5,
    InvalidProof = 6,
    ThresholdNotMet = 7,
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

#[contracttype]
#[derive(Clone)]
pub struct Config {
    pub root: BytesN<32>,
    pub depth: u32,
    pub threshold: u32,
    pub vk: VkData,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    /// 32-byte action digest the approvals authorize (bound as the Semaphore signal).
    pub action: BytesN<32>,
    /// Unique-per-proposal scope seed (binds nullifiers to this proposal).
    pub scope_seed: BytesN<32>,
    pub approvals: u32,
    pub executed: bool,
}

#[contracttype]
pub struct ProofEntry {
    pub a: BytesN<64>,
    pub b: BytesN<128>,
    pub c: BytesN<64>,
    pub nullifier: BytesN<32>,
}

#[contracttype]
pub enum DataKey {
    Config,
    ProposalCount,
    Proposal(u32),
    Approval(u32, BytesN<32>),
}

#[contract]
pub struct ConfidentialDao;

#[contractimpl]
impl ConfidentialDao {
    pub fn __constructor(env: Env, root: BytesN<32>, depth: u32, threshold: u32, vk: VkData) {
        env.storage()
            .instance()
            .set(&DataKey::Config, &Config { root, depth, threshold, vk });
    }

    /// Open a proposal to authorize `action`; `scope_seed` must be unique per proposal.
    pub fn create_proposal(env: Env, action: BytesN<32>, scope_seed: BytesN<32>) -> u32 {
        let id: u32 = env.storage().instance().get(&DataKey::ProposalCount).unwrap_or(0);
        let p = Proposal { action, scope_seed, approvals: 0, executed: false };
        env.storage().persistent().set(&DataKey::Proposal(id), &p);
        env.storage().instance().set(&DataKey::ProposalCount, &(id + 1));
        env.events().publish((symbol_short!("proposed"), id), ());
        id
    }

    /// Approve a proposal with one Semaphore membership proof. Returns the new approval count.
    pub fn approve(env: Env, proposal_id: u32, entry: ProofEntry) -> Result<u32, Error> {
        let cfg: Config = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;
        let mut p: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;
        if p.executed {
            return Err(Error::AlreadyExecuted);
        }
        if !is_canonical(&entry.nullifier) {
            return Err(Error::NonCanonicalInput);
        }

        // message := hash(action), scope := hash(scope_seed): bind to this action & proposal.
        let msg = semaphore_hash(&env, &p.action);
        let scope = semaphore_hash(&env, &p.scope_seed);
        let pub_signals = soroban_sdk::vec![
            &env,
            cfg.root.clone(),
            entry.nullifier.clone(),
            msg,
            scope
        ];
        let ok = verify_encoded(
            &env,
            &pub_signals,
            &entry.a,
            &entry.b,
            &entry.c,
            &cfg.vk.alpha,
            &cfg.vk.beta,
            &cfg.vk.gamma,
            &cfg.vk.delta,
            &cfg.vk.ic,
        );
        if !ok {
            return Err(Error::InvalidProof);
        }

        let akey = DataKey::Approval(proposal_id, entry.nullifier.clone());
        if env.storage().persistent().has(&akey) {
            return Err(Error::AlreadyApproved);
        }
        env.storage().persistent().set(&akey, &());
        p.approvals += 1;
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &p);
        env.events().publish((symbol_short!("approved"), proposal_id), p.approvals);
        Ok(p.approvals)
    }

    /// Execute a proposal once it has reached the threshold (marks it executed).
    pub fn execute(env: Env, proposal_id: u32) -> Result<(), Error> {
        let cfg: Config = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;
        let mut p: Proposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;
        if p.executed {
            return Err(Error::AlreadyExecuted);
        }
        if p.approvals < cfg.threshold {
            return Err(Error::ThresholdNotMet);
        }
        p.executed = true;
        env.storage().persistent().set(&DataKey::Proposal(proposal_id), &p);
        env.events().publish((symbol_short!("executed"), proposal_id), p.action);
        Ok(())
    }

    pub fn proposal(env: Env, proposal_id: u32) -> Result<Proposal, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)
    }
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod fixture;
#[cfg(test)]
mod test;
