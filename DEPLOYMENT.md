# Testnet deployment

The proposal-based confidential DAO is deployed and exercised end-to-end live on Stellar
**testnet**. (The verifier it depends on is deployed from the core library repo.)

| | |
|---|---|
| `ConfidentialDao` | `CBJYJ3Z35NZO3HQMJCWVH3PEW5IW3AOVLHPUI66G5BQPGYYRK65I4EB7` |
| Flow | `create_proposal` → `approve` (A) → `approve` (B) → `execute` |
| Result | proposal `{approvals: 2, executed: true}`, `executed` event emitted |
| Double-approve same proposal | rejected `Error(Contract, #4)` = `AlreadyApproved` |
| Approve an executed proposal | rejected `Error(Contract, #3)` = `AlreadyExecuted` |

Run it: `scripts/dao-flow.sh`.

The single-tx k-of-n account (`contracts/multisig`) and the single-signer account
(`contracts/account`) are unit-tested against real proofs; their on-chain `__check_auth`
auth-entry flow is driven from the test harness and scripts.

## On-chain `__check_auth` (the core) — verified on testnet

A `SemaphoreAccount` authorizes a real transaction with an anonymous Semaphore proof (no key,
no signature), while a different account pays the fee (payer-privacy). See
`tools/onchain-checkauth/`.

| | |
|---|---|
| Account | `CBUGFMOHJC7RDS4ZGABTRHTBZJVH766TSHC3ROR5GMFLXBOALKRUQTTG` |
| `init` | [`9d677f55…`](https://stellar.expert/explorer/testnet/tx/9d677f551f77b725761cd7db654dac9e9275c7f4889aafc96ca5cbcc8ab8887f) |
| `protected()` authorized by a Semaphore proof | [`9a5462f7…`](https://stellar.expert/explorer/testnet/tx/9a5462f7b128fa29f06efcc44592bcf182da4834c3393356e4eb72d9c10ad39a) → `SUCCESS`, returns `42` |

The proof is generated at auth time over the exact `signature_payload` the host checks, and
returned as the account's custom signature via `authorizeEntry({ signatureScVal })`.

## On-chain `__check_auth` for the k-of-n multisig — verified on testnet

Two distinct members each prove over the runtime auth payload; the `Vec<ProofEntry>` is the
account signature. `__check_auth` verifies both proofs, enforces distinct nullifiers and the
threshold, and `protected()` executes. See `tools/onchain-checkauth/multisig-*.mjs`.

| | |
|---|---|
| Multisig (2-of-3) | `CBI4XIQLTURVQWWSPLKQF2E27FULKY5462S6GIAUFJTDMOTNJU7YGVA6` |
| 2-of-3 `__check_auth` | [`8836726b…`](https://stellar.expert/explorer/testnet/tx/8836726b84da0c97d184991b32eb35928346c83cc55eb475dd23f8ea07dfef1a) → `SUCCESS`, returns `42` |

## Explicit fee-bump payer-privacy — verified on testnet

An inner tx (source A) invokes the account's `protected()` authorized by the member's ZK proof;
a distinct relayer B pays the fee via a Stellar fee-bump. See `tools/onchain-checkauth/fee-bump.mjs`.

| | |
|---|---|
| Fee-bump tx | [`18c4a43b…`](https://stellar.expert/explorer/testnet/tx/18c4a43b80e84a9e5af3650070346a50b70142c5b174ee5cef699abe9f60e67e) → `SUCCESS` |
| Inner source A paid | `0.0000000 XLM` |
| Relayer B paid (the fee) | `0.0030968 XLM` |

The member authorizes purely in zero-knowledge — no account, no key — and a different party pays.
