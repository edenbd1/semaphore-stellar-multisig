// Phase 3: drive a REAL on-chain __check_auth. Invoke the account's protected(), simulate to
// get the auth payload the host will check, generate a Semaphore proof bound to THAT payload,
// return it as the custom account signature, and submit. The account authorizes with a
// zero-knowledge membership proof — no key, no signature, and the fee is paid by a different
// account (payer-privacy).
import { Identity } from "@semaphore-protocol/identity"
import { Group } from "@semaphore-protocol/group"
import { generateProof, unpackGroth16Proof } from "@semaphore-protocol/proof"
import { readFileSync } from "fs"
import * as pkg from "@stellar/stellar-sdk"
const { rpc, TransactionBuilder, Operation, Keypair, Networks, Address, authorizeEntry, xdr, scValToNative } = pkg

const ACCOUNT_ID = process.argv[2]
const RPC = "https://soroban-testnet.stellar.org"
const PASSPHRASE = Networks.TESTNET

const st = JSON.parse(readFileSync("/tmp/account-e2e.json", "utf8"))
const member = await Identity.import(st.identity)
const group = new Group(st.commitments.map((c) => BigInt(c)))
const scopeBig = BigInt(st.scopeSeed)
const source = Keypair.fromSecret(readFileSync("/tmp/semdev.secret", "utf8").trim())

const be32 = (d) => BigInt(d).toString(16).padStart(64, "0")
const buf = (hex) => Buffer.from(hex, "hex")
const bytes = (b) => xdr.ScVal.scvBytes(b)
const entryScVal = (a, b, c, nul) =>
    xdr.ScVal.scvMap([
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("a"), val: bytes(a) }),
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("b"), val: bytes(b) }),
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("c"), val: bytes(c) }),
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("nullifier"), val: bytes(nul) }),
    ])

const server = new rpc.Server(RPC, { allowHttp: false })

async function sign(_preimage, payload) {
    const message = BigInt("0x" + Buffer.from(payload).toString("hex"))
    console.log(">> auth payload:", Buffer.from(payload).toString("hex").slice(0, 24), "... generating proof over it")
    const proof = await generateProof(member, group, message, scopeBig)
    const u = unpackGroth16Proof(proof.points)
    const a = buf(be32(u.pi_a[0]) + be32(u.pi_a[1]))
    const b = buf(be32(u.pi_b[0][1]) + be32(u.pi_b[0][0]) + be32(u.pi_b[1][1]) + be32(u.pi_b[1][0]))
    const c = buf(be32(u.pi_c[0]) + be32(u.pi_c[1]))
    const nul = buf(be32(proof.nullifier))
    console.log(">> proof ready, nullifier", be32(proof.nullifier).slice(0, 16))
    return { signatureScVal: entryScVal(a, b, c, nul) }
}

function invokeOp(auth) {
    return Operation.invokeContractFunction({ contract: ACCOUNT_ID, function: "protected", args: [], auth })
}

const acct = await server.getAccount(source.publicKey())
const tx = new TransactionBuilder(acct, { fee: "1000000", networkPassphrase: PASSPHRASE })
    .addOperation(invokeOp([]))
    .setTimeout(60)
    .build()

const sim = await server.simulateTransaction(tx)
if (rpc.Api.isSimulationError(sim)) throw new Error("sim error: " + sim.error)
const authEntries = sim.result.auth
console.log(">> simulation returned", authEntries.length, "auth entry(ies)")

const validUntil = (await server.getLatestLedger()).sequence + 100
const signed = []
for (const e of authEntries) signed.push(await authorizeEntry(e, sign, validUntil, PASSPHRASE))

const acct2 = await server.getAccount(source.publicKey())
let tx2 = new TransactionBuilder(acct2, { fee: "1000000", networkPassphrase: PASSPHRASE })
    .addOperation(invokeOp(signed))
    .setTimeout(60)
    .build()
// Re-simulate WITH the signed auth so the resource estimate includes __check_auth's pairing
// cost (the empty-auth simulation skips __check_auth and under-estimates CPU).
const sim2 = await server.simulateTransaction(tx2)
if (rpc.Api.isSimulationError(sim2)) throw new Error("sim2 error: " + sim2.error)
console.log(">> re-simulated with auth; min resource fee:", sim2.minResourceFee)
tx2 = rpc.assembleTransaction(tx2, sim2).build()
tx2.sign(source)

const res = await server.sendTransaction(tx2)
console.log(">> sent:", res.hash, res.status)
let g = await server.getTransaction(res.hash)
for (let i = 0; i < 30 && g.status === "NOT_FOUND"; i++) {
    await new Promise((r) => setTimeout(r, 2000))
    g = await server.getTransaction(res.hash)
}
console.log("================================")
console.log(">> FINAL STATUS:", g.status, "| tx:", res.hash)
try { console.log(">> returnValue:", g.returnValue ? scValToNative(g.returnValue) : "(n/a)") } catch { console.log(">> returnValue: (n/a)") }
console.log("================================")
process.exit(g.status === "SUCCESS" ? 0 : 1)
