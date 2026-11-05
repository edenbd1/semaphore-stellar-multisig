# On-chain `__check_auth` demo

Proves the end-to-end flow: a `SemaphoreAccount` authorizes a real transaction with an
anonymous Semaphore membership proof — no key, no signature — while a *different* account pays
the fee (payer-privacy).

```bash
npm install
# 1) generate a group + member, print init args, persist identity to /tmp/account-e2e.json
node setup.mjs > /tmp/account-init.json
# 2) deploy + init the account via the Stellar CLI with those args (root, scope_seed, vk)
#    (see the repo scripts), save the id to /tmp/account_id, and the source secret to /tmp/semdev.secret
# 3) invoke protected(): simulate -> get the auth payload -> prove over it -> submit
node auth.mjs "$(cat /tmp/account_id)"
```

How it works: `auth.mjs` simulates the call to learn the host's `signature_payload`, generates a
Semaphore proof whose `message` is bound to that payload, returns it as the account's custom
signature via `authorizeEntry({ signatureScVal })`, then **re-simulates with the signed auth** so
the resource estimate includes `__check_auth`'s pairing cost (the empty-auth simulation skips it
and under-estimates CPU → `RESOURCE_LIMIT_EXCEEDED`).

Verified on testnet: account `CBUGFMOHJC7RDS4ZGABTRHTBZJVH766TSHC3ROR5GMFLXBOALKRUQTTG`,
tx `9a5462f7b128fa29f06efcc44592bcf182da4834c3393356e4eb72d9c10ad39a` → `SUCCESS`, returns `42`.

## Explicit fee-bump (payer-privacy, end to end)

`fee-bump.mjs` shows the full payer-privacy flow: an inner Soroban tx (source **A**) invokes the
account's `protected()`, authorized by the anonymous member's ZK proof; a **distinct relayer B**
wraps it in a Stellar fee-bump (CAP-15) and pays the fee. **A pays nothing, the member has no
key at all.**

```bash
node fee-bump.mjs "$(cat /tmp/fb_account_id)"
```

Verified on testnet (tx `18c4a43b80e84a9e5af3650070346a50b70142c5b174ee5cef699abe9f60e67e`,
`SUCCESS`): inner source A paid `0.0000000 XLM`, relayer B paid the fee `0.0030968 XLM`.
