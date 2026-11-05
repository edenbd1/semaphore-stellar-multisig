// Live testnet helper for the SemaphoreAccount (custom account, __check_auth) with domain sep.
//   node live-account.mjs root   -> {root, vk, scope_seed}  (member derived from fixed secret)
import { Identity } from "@semaphore-protocol/identity"
import { Group } from "@semaphore-protocol/group"
import { readFileSync } from "fs"

const VKEY = "/Users/eden/data/semaphore-stellar-multisig/fixtures/vkey-depth2.json"
const A = new Identity("account-live-member-A")
const B = new Identity("account-live-member-B")
const C = new Identity("account-live-member-C")
const group = new Group([A.commitment, B.commitment, C.commitment])
const SEED_HEX = "33".repeat(32)
const be32 = (d) => BigInt(d).toString(16).padStart(64, "0")
const G1 = (p) => be32(p[0]) + be32(p[1])
const G2 = (p) => be32(p[0][1]) + be32(p[0][0]) + be32(p[1][1]) + be32(p[1][0])

const vkey = JSON.parse(readFileSync(VKEY, "utf8"))
console.log(JSON.stringify({
    root: be32(group.root.toString()),
    scope_seed: SEED_HEX,
    vk: { alpha: G1(vkey.vk_alpha_1), beta: G2(vkey.vk_beta_2), gamma: G2(vkey.vk_gamma_2), delta: G2(vkey.vk_delta_2), ic: vkey.IC.map(G1) },
}))
