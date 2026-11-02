# On-chain `__check_auth` demo

Proves the end-to-end flow: a `SemaphoreAccount` authorizes a real transaction with an
anonymous Semaphore membership proof — no key, no signature — while a *different* account pays
the fee (payer-privacy).

```bash
npm install
# 1) generate a group + member, print init args, persist identity to /tmp/account-e2e.json
node setup.mjs > /tmp/account-init.json
# 2) deploy + init the account via the Stellar CLI with those args (root, scope_hash, vk)
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
