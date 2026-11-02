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
