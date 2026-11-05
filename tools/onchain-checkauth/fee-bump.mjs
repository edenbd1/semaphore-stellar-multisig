// Explicit fee-bump payer-privacy: an inner Soroban tx (source A) invokes the Semaphore
// account's protected(), authorized by the anonymous member's ZK proof; a DISTINCT relayer B
// wraps it in a fee-bump and pays the fee. A pays nothing; the member has no key at all.
import { Identity } from "@semaphore-protocol/identity"
import { Group } from "@semaphore-protocol/group"
import { generateProof, unpackGroth16Proof } from "@semaphore-protocol/proof"
import { keccak256, toBeHex, concat, toUtf8Bytes, getBytes } from "ethers"
import { readFileSync } from "fs"
import * as pkg from "@stellar/stellar-sdk"
const { rpc, Horizon, TransactionBuilder, Operation, Keypair, Networks, authorizeEntry, xdr, scValToNative } = pkg

const AID = process.argv[2]
const PASSPHRASE = Networks.TESTNET
const A = Keypair.fromSecret(readFileSync("/tmp/A.secret", "utf8").trim())   // inner tx source
const B = Keypair.fromSecret(readFileSync("/tmp/B.secret", "utf8").trim())   // relayer / fee source
const st = JSON.parse(readFileSync("/tmp/account-e2e.json", "utf8"))
const member = await Identity.import(st.identity)
const group = new Group(st.commitments.map((c) => BigInt(c)))
// Domain separation: the account derives its scope on-chain from its OWN address, so the proof's
// scope must fold the deployed account id (AID) in exactly the same way.
const scopeBig = BigInt(keccak256(concat([toUtf8Bytes(AID), getBytes(toBeHex(BigInt(st.scopeSeed), 32))])))

const server = new rpc.Server("https://soroban-testnet.stellar.org")
const horizon = new Horizon.Server("https://horizon-testnet.stellar.org")
const be32 = (d) => BigInt(d).toString(16).padStart(64, "0")
const buf = (h) => Buffer.from(h, "hex")
const bv = (x) => xdr.ScVal.scvBytes(x)
const native = async (pub) => (await horizon.loadAccount(pub)).balances.find((b) => b.asset_type === "native").balance

async function sign(_pre, payload) {
    const message = BigInt("0x" + Buffer.from(payload).toString("hex"))
    const p = await generateProof(member, group, message, scopeBig)
    const u = unpackGroth16Proof(p.points)
    const sig = xdr.ScVal.scvMap([
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("a"), val: bv(buf(be32(u.pi_a[0]) + be32(u.pi_a[1]))) }),
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("b"), val: bv(buf(be32(u.pi_b[0][1]) + be32(u.pi_b[0][0]) + be32(u.pi_b[1][1]) + be32(u.pi_b[1][0]))) }),
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("c"), val: bv(buf(be32(u.pi_c[0]) + be32(u.pi_c[1]))) }),
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("nullifier"), val: bv(buf(be32(p.nullifier))) }),
    ])
    return { signatureScVal: sig }
}
const op = (auth) => Operation.invokeContractFunction({ contract: AID, function: "protected", args: [], auth })

const [A0, B0] = [await native(A.publicKey()), await native(B.publicKey())]
console.log(">> before:  A =", A0, "XLM |  B(relayer) =", B0, "XLM")

// inner tx: source A, ZK-authorized protected()
let inner = new TransactionBuilder(await server.getAccount(A.publicKey()), { fee: "1000000", networkPassphrase: PASSPHRASE }).addOperation(op([])).setTimeout(120).build()
const sim = await server.simulateTransaction(inner)
if (rpc.Api.isSimulationError(sim)) throw new Error("sim: " + sim.error)
const validUntil = (await server.getLatestLedger()).sequence + 100
const signed = []
for (const e of sim.result.auth) signed.push(await authorizeEntry(e, sign, validUntil, PASSPHRASE))
let inner2 = new TransactionBuilder(await server.getAccount(A.publicKey()), { fee: "1000000", networkPassphrase: PASSPHRASE }).addOperation(op(signed)).setTimeout(120).build()
const sim2 = await server.simulateTransaction(inner2)
if (rpc.Api.isSimulationError(sim2)) throw new Error("sim2: " + sim2.error)
inner2 = rpc.assembleTransaction(inner2, sim2).build()
inner2.sign(A) // A signs the inner tx (sequence source) — but pays NO fee

// fee-bump: B pays the fee
const feeBump = TransactionBuilder.buildFeeBumpTransaction(B, inner2.fee, inner2, PASSPHRASE)
feeBump.sign(B)
console.log(">> inner source = A, fee source = B; inner fee field:", inner2.fee, "stroops (paid by B via bump)")

const res = await server.sendTransaction(feeBump)
let g = await server.getTransaction(res.hash)
for (let i = 0; i < 40 && g.status === "NOT_FOUND"; i++) { await new Promise((r) => setTimeout(r, 2000)); g = await server.getTransaction(res.hash) }
console.log("================================")
console.log(">> FEE-BUMP STATUS:", g.status, "| tx:", res.hash)
const [A1, B1] = [await native(A.publicKey()), await native(B.publicKey())]
console.log(">> after:   A =", A1, "XLM |  B(relayer) =", B1, "XLM")
console.log(">> A paid:", (A0 - A1).toFixed(7), "XLM  |  B paid:", (B0 - B1).toFixed(7), "XLM")
console.log("================================")
process.exit(g.status === "SUCCESS" ? 0 : 1)
