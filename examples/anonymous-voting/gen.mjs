// Generate a voting committee (real Semaphore identities) and one anonymous YES vote per member
// for a given proposal, encoded for the ConfidentialDao contract. Writes voting-args.json.
import { Identity } from "@semaphore-protocol/identity"
import { Group } from "@semaphore-protocol/group"
import { generateProof, unpackGroth16Proof } from "@semaphore-protocol/proof"
import { keccak256, toUtf8Bytes, toBeHex } from "ethers"
import { writeFileSync } from "fs"

const COMMITTEE_SIZE = 3
const THRESHOLD = 2
const PROPOSAL = "Fund 10,000 XLM to the community grants pool"

// Proposal action digest + a unique per-proposal scope seed (both 32-byte field values).
const action = BigInt(keccak256(toUtf8Bytes(PROPOSAL)))
const scopeSeed = BigInt(keccak256(toUtf8Bytes("proposal-1:" + PROPOSAL)))

const hash = (m) => (BigInt(keccak256(toBeHex(m, 32))) >> 8n).toString()
const be32 = (d) => BigInt(d).toString(16).padStart(64, "0")
const G1 = (p) => be32(p[0]) + be32(p[1])
const G2 = (p) => be32(p[0][1]) + be32(p[0][0]) + be32(p[1][1]) + be32(p[1][0])

const members = Array.from({ length: COMMITTEE_SIZE }, () => new Identity())
const group = new Group(members.map((m) => m.commitment))

console.log(`>> committee of ${COMMITTEE_SIZE}, threshold ${THRESHOLD}`)
console.log(`>> proposal: "${PROPOSAL}"`)

// each member casts an anonymous YES: a Semaphore proof bound to the proposal (message=action,
// scope=scopeSeed). Distinct members => distinct nullifiers => at most one vote each.
const votes = []
for (let i = 0; i < members.length; i++) {
    const p = await generateProof(members[i], group, action, scopeSeed)
    const u = unpackGroth16Proof(p.points)
    votes.push({
        a: G1([u.pi_a[0], u.pi_a[1]]),
        b: G2(u.pi_b),
        c: G1([u.pi_c[0], u.pi_c[1]]),
        nullifier: be32(p.nullifier),
    })
    console.log(`>> member ${i + 1} cast an anonymous vote (nullifier ${be32(p.nullifier).slice(0, 12)}...)`)
}

const vkey = await (await fetch("https://snark-artifacts.pse.dev/semaphore/4.13.0/semaphore-2.json")).json()
const vk = {
    alpha: G1(vkey.vk_alpha_1),
    beta: G2(vkey.vk_beta_2),
    gamma: G2(vkey.vk_gamma_2),
    delta: G2(vkey.vk_delta_2),
    ic: vkey.IC.map(G1),
}

writeFileSync("voting-args.json", JSON.stringify({
    root: be32(group.root),
    depth: 2,
    threshold: THRESHOLD,
    action: be32(action),
    scope_seed: be32(scopeSeed),
    proposal: PROPOSAL,
    vk,
    votes,
}, null, 2))
console.log(`>> wrote voting-args.json (group root ${be32(group.root).slice(0, 12)}...)`)
