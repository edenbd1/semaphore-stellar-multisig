import { Identity } from "@semaphore-protocol/identity"
import { Group } from "@semaphore-protocol/group"
import { readFileSync, writeFileSync } from "fs"
const be32 = (d) => BigInt(d).toString(16).padStart(64, "0")
const m1 = new Identity(), m2 = new Identity(), m3 = new Identity()
const group = new Group([m1.commitment, m2.commitment, m3.commitment])
const vkey = JSON.parse(readFileSync("new URL("../../fixtures/vkey-depth2.json", import.meta.url)", "utf8"))
const G1 = (p) => be32(p[0]) + be32(p[1])
const G2 = (p) => be32(p[0][1]) + be32(p[0][0]) + be32(p[1][1]) + be32(p[1][0])
const vk = { alpha: G1(vkey.vk_alpha_1), beta: G2(vkey.vk_beta_2), gamma: G2(vkey.vk_gamma_2), delta: G2(vkey.vk_delta_2), ic: vkey.IC.map(G1) }
writeFileSync("/tmp/msig-e2e.json", JSON.stringify({
    members: [m1.export(), m2.export()], // the two signers (threshold 2)
    commitments: [m1.commitment.toString(), m2.commitment.toString(), m3.commitment.toString()],
    root: group.root.toString(),
}))
console.log(JSON.stringify({ root: be32(group.root), depth: 2, threshold: 2, vk }))
