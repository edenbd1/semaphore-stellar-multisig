// On-chain __check_auth for the confidential k-of-n multisig: two distinct members each prove
// over the runtime auth payload; the Vec<ProofEntry> is the account signature.
import { Identity } from "@semaphore-protocol/identity"
import { Group } from "@semaphore-protocol/group"
import { generateProof, unpackGroth16Proof } from "@semaphore-protocol/proof"
import { readFileSync } from "fs"
import * as pkg from "@stellar/stellar-sdk"
const { rpc, TransactionBuilder, Operation, Keypair, Networks, authorizeEntry, xdr, scValToNative } = pkg

const MID = process.argv[2]
const PASSPHRASE = Networks.TESTNET
const st = JSON.parse(readFileSync("/tmp/msig-e2e.json", "utf8"))
const members = await Promise.all(st.members.map((m) => Identity.import(m)))
const group = new Group(st.commitments.map((c) => BigInt(c)))
const source = Keypair.fromSecret(readFileSync("/tmp/semdev.secret", "utf8").trim())
const server = new rpc.Server("https://soroban-testnet.stellar.org")

const be32 = (d) => BigInt(d).toString(16).padStart(64, "0")
const buf = (h) => Buffer.from(h, "hex")
const b = (x) => xdr.ScVal.scvBytes(x)
const entry = (a, bb, c, nul) => xdr.ScVal.scvMap([
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("a"), val: b(a) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("b"), val: b(bb) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("c"), val: b(c) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("nullifier"), val: b(nul) }),
])

async function proofFor(id, message) {
    const p = await generateProof(id, group, message, message) // message == scope == payload
    const u = unpackGroth16Proof(p.points)
    return entry(
        be32(u.pi_a[0]) + be32(u.pi_a[1]),
        be32(u.pi_b[0][1]) + be32(u.pi_b[0][0]) + be32(u.pi_b[1][1]) + be32(u.pi_b[1][0]),
        be32(u.pi_c[0]) + be32(u.pi_c[1]),
        be32(p.nullifier),
    )
}

async function sign(_pre, payload) {
    const message = BigInt("0x" + Buffer.from(payload).toString("hex"))
    console.log(">> payload", Buffer.from(payload).toString("hex").slice(0, 20), "- proving for", members.length, "members")
    const entries = []
    for (const m of members) entries.push(await proofFor(m, message))
    return { signatureScVal: xdr.ScVal.scvVec(entries) }
}

const op = (auth) => Operation.invokeContractFunction({ contract: MID, function: "protected", args: [], auth })
const acct = await server.getAccount(source.publicKey())
const tx = new TransactionBuilder(acct, { fee: "2000000", networkPassphrase: PASSPHRASE }).addOperation(op([])).setTimeout(60).build()
const sim = await server.simulateTransaction(tx)
if (rpc.Api.isSimulationError(sim)) throw new Error("sim: " + sim.error)
const validUntil = (await server.getLatestLedger()).sequence + 100
const signed = []
for (const e of sim.result.auth) signed.push(await authorizeEntry(e, sign, validUntil, PASSPHRASE))
const acct2 = await server.getAccount(source.publicKey())
let tx2 = new TransactionBuilder(acct2, { fee: "2000000", networkPassphrase: PASSPHRASE }).addOperation(op(signed)).setTimeout(60).build()
const sim2 = await server.simulateTransaction(tx2)
if (rpc.Api.isSimulationError(sim2)) throw new Error("sim2: " + sim2.error)
console.log(">> re-sim min resource fee:", sim2.minResourceFee)
tx2 = rpc.assembleTransaction(tx2, sim2).build(); tx2.sign(source)
const res = await server.sendTransaction(tx2)
let g = await server.getTransaction(res.hash)
for (let i = 0; i < 30 && g.status === "NOT_FOUND"; i++) { await new Promise(r => setTimeout(r, 2000)); g = await server.getTransaction(res.hash) }
console.log("================================")
console.log(">> FINAL:", g.status, "| tx:", res.hash, "| returns:", g.returnValue ? scValToNative(g.returnValue) : "n/a")
console.log("================================")
process.exit(g.status === "SUCCESS" ? 0 : 1)
