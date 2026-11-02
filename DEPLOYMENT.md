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
