# Semaphore for Stellar — Confidential Multisig

Confidential threshold multisig for Soroban/Stellar, built on the
[semaphore-stellar](https://github.com/edenbd1/semaphore-stellar) library (a faithful
Semaphore v4 port, BN254, reusing Ethereum's trusted-setup ceremony).

Members are a Semaphore group; authorization is an anonymous membership proof, so you learn
that `k` distinct members approved — never *which* ones.

## Contracts

- **`contracts/account`** — single-signer custom account (`__check_auth`): one Semaphore proof
  authorizes, with the signal bound to `signature_payload` and a nullifier replay guard.
- **`contracts/multisig`** — confidential **k-of-n** account (`__check_auth`, single tx): `k`
  distinct-nullifier proofs, `k >= threshold`; budget ~29M instructions per proof (so up to ~13/tx on CPU, against the 400M per-tx limit).
- **`contracts/dao`** — proposal-based confidential multisig: members approve asynchronously
  (one proof per tx, no per-tx ceiling); `create_proposal` → `approve` → `execute`.
- **`contracts/verifier`** — vendored Groth16/BN254 verifier (see `VENDORED.md`).

## Test

```bash
cargo test            # 15 tests (verifier 3, account 3, multisig 5, dao 4)
stellar contract build
```

The `dao` flow runs end-to-end live on testnet (see the library repo's deployment notes and
`scripts/`). License: MIT.

## Example: Anonymous Voting (flagship)

[`examples/anonymous-voting`](./examples/anonymous-voting) runs a full anonymous vote on Stellar testnet: a committee votes on a proposal, and at threshold it executes, without revealing who voted. Live run: contract `CCCIIY6D...`.

## Status & safety

> ⚠️ **Prototype — unaudited, testnet only.** This code has **not** been audited and runs only
> on Stellar **testnet** with small fixtures. Do not use it with real value or on mainnet.

### Known limitations

- **Domain separation (enforced):** every contract derives its scope **on-chain from its own contract address** (`domain_scope_hash` = `keccak256(ascii(contract_strkey) ‖ scope_seed) >> 8`), so a proof minted for one deployment **cannot** be replayed against another that shares a group root — even with an identical `(group root, scope seed)`. Each repo has a `rejects_cross_contract_replay` test proving this against a real ceremony proof.
- `account`: the scope is fixed at construction, so each member can authorize the account **once ever**, not once per transaction. A per-tx-scope variant (like `multisig`) is planned.
- `dao`: `create_proposal`/`execute` are permissionless (griefing / storage growth), and a **self-submitted** approval's fee-payer is public — route approvals through a relayer to keep the voter anonymous.
- Storage TTL is managed: every state-changing call extends the instance config and the relevant persistent entries (nullifiers, proposals, approvals, scope-seed markers) by ~30 days (bumping when under ~15 days remain), so an actively-used contract never archives. A contract left completely idle past the bump window still needs a one-time Soroban `restore`.
> Proofs reuse Ethereum's Semaphore trusted-setup ceremony (no new ceremony is performed).
