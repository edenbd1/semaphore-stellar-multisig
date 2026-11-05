# On-chain `__check_auth` demo (domain-separated)

Proves the end-to-end flow: a `SemaphoreAccount` authorizes a real transaction with an
anonymous Semaphore membership proof — no key, no signature — and the scope is **bound to the
account's own contract address** (domain separation), so the proof is valid only for *this*
deployment.

Run the `node` steps from a dir that resolves `@semaphore-protocol/*`, `@stellar/stellar-sdk`
(v14+, for `authorizeEntry` + `rpc`) and `ethers` (e.g. the ceremony checkout). Put the tx
source/fee-payer secret in `/tmp/feemeasure.secret`.

```bash
# 1) deploy the account with (root, scope_seed, vk) — see live-account.mjs for the deploy args
node ../live-account.mjs                 # prints {root, scope_seed, vk}
#    deploy via the Stellar CLI, note the deployed id (AID)
# 2) ZK-authorize protected() on THAT account (scope folds AID in):
node account-auth-domain.mjs <AID>
```

How it works: the script simulates the call to learn the host's `signature_payload`, generates a
Semaphore proof whose `message` is bound to that payload **and** whose `scope` is
`keccak256(ascii(AID) ‖ scope_seed) >> 8` (matching the on-chain `domain_scope_hash`), returns it as
the account's custom signature via `authorizeEntry({ signatureScVal })`, then re-simulates with the
signed auth so the resource estimate includes `__check_auth`'s pairing cost.

Verified on testnet: account `CATRB54VSE4XOTWQN5YW4ZNYQRGF2SMXQBHY2UAHIXHG5NF3BFZW7RAR`,
tx `e47162c4e8a33b97a83d488344eafa465c1fc1e7222b62b3442dcbcd6f97054a` → `SUCCESS`, returns `42`.

## Confidential k-of-n multisig

`multisig-auth-domain.mjs` proves the same for the threshold multisig: `k` distinct members each
prove over the runtime payload, the `Vec<ProofEntry>` is the account signature, and every proof's
scope is domain-bound to the multisig address.

```bash
node multisig-auth-domain.mjs <MID>
```

Verified on testnet: multisig `CCYKU4KL6XFGRQ7T7ET724VC7LLKMEWUYDR4HRI4DGDXCZJQU5AWHOZT`,
tx `38b0b7b8734e92fa16c1c73858bea3b32de66e28c090219e4e06374afa4e74fd` → `SUCCESS`, returns `42`.

## Explicit fee-bump (payer-privacy, end to end)

`fee-bump.mjs` shows the full payer-privacy flow: an inner Soroban tx (source **A**) invokes the
account's `protected()`, authorized by the anonymous member's ZK proof (scope bound to AID); a
**distinct relayer B** wraps it in a Stellar fee-bump (CAP-15) and pays the fee. **A pays nothing,
the member has no key at all.**

```bash
node fee-bump.mjs "$(cat /tmp/fb_account_id)"
```

Prior verified run (pre-domain account) on testnet:
tx `18c4a43b80e84a9e5af3650070346a50b70142c5b174ee5cef699abe9f60e67e`, `SUCCESS`: inner source A
paid `0.0000000 XLM`, relayer B paid the fee `0.0030968 XLM`.
