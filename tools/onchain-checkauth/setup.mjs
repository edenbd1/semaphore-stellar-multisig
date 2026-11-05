// Phase 1 of the on-chain __check_auth demo: generate a fresh Semaphore group + member, pick a
// per-account scope seed, and print the init args (root, scope_seed, vk) for the CLI. Persist
// the member identity + group + scope seed so phase 3 can prove over the runtime auth payload.
//
// NOTE (domain separation): the account now derives its scope ON-CHAIN as
//   scope = keccak256( ascii(deployed_contract_strkey) || scope_seed ) >> 8
// so the constructor takes the raw `scope_seed` (below), and phase 3 must fold the DEPLOYED
// contract address into the proof's scope — see tools/gen-account-fixture-domain.mjs for the
// exact recipe. A proof made for one account no longer verifies against another.
import { Identity } from "@semaphore-protocol/identity"
import { Group } from "@semaphore-protocol/group"
import { keccak256, toBeHex } from "ethers"
import { readFileSync, writeFileSync } from "fs"

const hash = (m) => (BigInt(keccak256(toBeHex(m, 32))) >> 8n).toString()
const be32 = (d) => BigInt(d).toString(16).padStart(64, "0")

const member = new Identity()
const other1 = new Identity()
const other2 = new Identity()
const group = new Group([member.commitment, other1.commitment, other2.commitment])

const SCOPE_SEED = "0x" + "33".repeat(32) // per-account scope seed (raw; scope is derived on-chain)
const root = group.root.toString()

const vkey = JSON.parse(readFileSync("new URL("../../fixtures/vkey-depth2.json", import.meta.url)", "utf8"))
const G1 = (p) => be32(p[0]) + be32(p[1])
const G2 = (p) => be32(p[0][1]) + be32(p[0][0]) + be32(p[1][1]) + be32(p[1][0])
const vk = {
    alpha: G1(vkey.vk_alpha_1),
    beta: G2(vkey.vk_beta_2),
    gamma: G2(vkey.vk_gamma_2),
    delta: G2(vkey.vk_delta_2),
    ic: vkey.IC.map(G1),
}

// persist for phase 3 (reconstruct the identity + group, prove over the runtime payload)
writeFileSync("/tmp/account-e2e.json", JSON.stringify({
    identity: member.export(),
    commitments: [member.commitment.toString(), other1.commitment.toString(), other2.commitment.toString()],
    scopeSeed: SCOPE_SEED,
    root,
}))

// init args for the CLI (constructor now takes the raw scope_seed; scope is derived on-chain)
console.log(JSON.stringify({ root: be32(root), scope_seed: BigInt(SCOPE_SEED).toString(16).padStart(64, "0"), vk }))
