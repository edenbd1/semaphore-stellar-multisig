# Anonymous Voting on Stellar

A flagship demo of [Semaphore for Stellar](https://github.com/edenbd1/semaphore-stellar):
a committee votes on a proposal **anonymously**, and the result is enforced on-chain.

## The problem

On-chain governance is public by default: every vote is tied to an address. That enables
coercion, vote-buying, and retaliation. Making votes anonymous while still guaranteeing
"one member, one vote" and a verifiable tally is exactly what Semaphore is for.

## How it works

The committee is a Semaphore group. To vote, each member submits a zero-knowledge proof that
they belong to the group, bound to the proposal, carrying a **nullifier** that is deterministic
in `(proposal, member)`. The on-chain `ConfidentialDao` contract (in `../../contracts/dao`):

- verifies each proof against the group root and the ceremony verification key;
- requires the nullifiers to be **distinct**, so each member can vote at most once per proposal;
- counts approvals and executes the proposal once the **threshold** is reached.

So the chain learns that *k distinct members approved*, and nothing about *which* members. The
proofs reuse Ethereum's Semaphore trusted-setup ceremony unchanged (Stellar's BN254 is the same
curve), so no new ceremony is needed.

## Run it (live on testnet)

```bash
cd examples/anonymous-voting
./run.sh            # deps + a real anonymous vote on Stellar testnet
```

It generates a committee of 3 real Semaphore identities, deploys a fresh voting contract, opens a
proposal, casts anonymous votes until the 2-of-3 threshold is reached, and executes, printing the
contract id and the on-chain state.

## Live run (testnet)

A real run of this demo: a 3-member committee with a 2-of-3 threshold.

| | |
|---|---|
| Voting contract (`ConfidentialDao`) | `CCCIIY6DJR67YX325ODXVSOCVRWCSR32FI4GT5TTTODVHZNO7V3P5EMF` |
| Deploy tx | [`ffb17a10...`](https://stellar.expert/explorer/testnet/tx/ffb17a10c9ed93c66922abea5f4f473174e75c8d13ebd936f8e236a2fe6257df) |
| Flow | open proposal, two anonymous votes reach the threshold, execute |
| Final on-chain state | `{ approvals: 2, executed: true }` |

The chain shows two distinct members approved and the proposal executed, without revealing
which members voted. All vote transactions are visible on the contract's
[explorer page](https://stellar.expert/explorer/testnet/contract/CCCIIY6DJR67YX325ODXVSOCVRWCSR32FI4GT5TTTODVHZNO7V3P5EMF).


## Status

Prototype, unaudited, testnet only. Built on `semaphore-stellar-multisig` (the `dao` contract)
and the `semaphore-stellar` library. MIT.
