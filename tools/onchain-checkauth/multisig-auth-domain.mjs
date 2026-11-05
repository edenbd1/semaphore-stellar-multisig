// Live __check_auth for the confidential k-of-n multisig WITH domain separation.
//   node multisig-auth-domain.mjs <MID>
// Two distinct members each prove over the runtime payload; scope is bound to the multisig's
// OWN address MID (domain_scope_hash). The Vec<ProofEntry> is the account signature.
import { Identity } from "@semaphore-protocol/identity"
import { Group } from "@semaphore-protocol/group"
import { generateProof, unpackGroth16Proof } from "@semaphore-protocol/proof"
import { keccak256, toBeHex, concat, toUtf8Bytes, getBytes } from "ethers"
import { readFileSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import * as pkg from "@stellar/stellar-sdk"
const { rpc, TransactionBuilder, Operation, Keypair, Networks, authorizeEntry, xdr, scValToNative } = pkg

const MID = process.argv[2]
if (!/^C[A-Z0-9]{55}$/.test(MID)) throw new Error("bad MID: " + MID)
const PASSPHRASE = Networks.TESTNET
const SRC = Keypair.fromSecret(readFileSync("/tmp/feemeasure.secret", "utf8").trim())
const ART = join(tmpdir(), "snark-artifacts", "semaphore", "4.13.0")
const SNARK = { wasm: join(ART, "semaphore-2.wasm"), zkey: join(ART, "semaphore-2.zkey") }

// Same deterministic group the multisig was deployed with (live-dao.mjs identities).
const A = new Identity("dao-live-member-A")
const B = new Identity("dao-live-member-B")
const C = new Identity("dao-live-member-C")
const group = new Group([A.commitment, B.commitment, C.commitment])
const members = [A, B] // threshold 2

const be32 = (d) => BigInt(d).toString(16).padStart(64, "0")
const buf = (h) => Buffer.from(h, "hex")
const b = (x) => xdr.ScVal.scvBytes(x)
const entry = (a, bb, c, nul) => xdr.ScVal.scvMap([
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("a"), val: b(a) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("b"), val: b(bb) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("c"), val: b(c) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("nullifier"), val: b(nul) }),
])

async function proofFor(id, message, scopeBig) {
    const p = await generateProof(id, group, message, scopeBig, 2, SNARK)
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
    // scope = domain_scope_hash(MID, payload): payload is the multisig's scope seed here.
    const scopeBig = BigInt(keccak256(concat([toUtf8Bytes(MID), getBytes("0x" + Buffer.from(payload).toString("hex"))])))
    const entries = []
    for (const m of members) entries.push(await proofFor(m, message, scopeBig))
    return { signatureScVal: xdr.ScVal.scvVec(entries) }
}

const server = new rpc.Server("https://soroban-testnet.stellar.org")
const op = (auth) => Operation.invokeContractFunction({ contract: MID, function: "protected", args: [], auth })
const tx = new TransactionBuilder(await server.getAccount(SRC.publicKey()), { fee: "2000000", networkPassphrase: PASSPHRASE }).addOperation(op([])).setTimeout(60).build()
const sim = await server.simulateTransaction(tx)
if (rpc.Api.isSimulationError(sim)) throw new Error("sim: " + sim.error)
const validUntil = (await server.getLatestLedger()).sequence + 100
const signed = []
for (const e of sim.result.auth) signed.push(await authorizeEntry(e, sign, validUntil, PASSPHRASE))
let tx2 = new TransactionBuilder(await server.getAccount(SRC.publicKey()), { fee: "2000000", networkPassphrase: PASSPHRASE }).addOperation(op(signed)).setTimeout(60).build()
const sim2 = await server.simulateTransaction(tx2)
if (rpc.Api.isSimulationError(sim2)) throw new Error("sim2: " + sim2.error)
tx2 = rpc.assembleTransaction(tx2, sim2).build(); tx2.sign(SRC)
const res = await server.sendTransaction(tx2)
let g = await server.getTransaction(res.hash)
for (let i = 0; i < 40 && g.status === "NOT_FOUND"; i++) { await new Promise(r => setTimeout(r, 2000)); g = await server.getTransaction(res.hash) }
console.log("MULTISIG CHECK_AUTH:", g.status, "| tx:", res.hash, "| returns:", g.status === "SUCCESS" ? scValToNative(g.returnValue) : "n/a")
process.exit(g.status === "SUCCESS" ? 0 : 1)
