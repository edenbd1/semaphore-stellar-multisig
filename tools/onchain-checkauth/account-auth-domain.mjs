// Live __check_auth demo for SemaphoreAccount WITH domain separation.
//   node account-auth-domain.mjs <AID>
// feemeasure is the tx source + fee payer; the account's auth is a Semaphore proof whose scope
// is bound to the account's OWN address AID (domain_scope_hash), proving the on-chain derivation
// matches the prover on the real network.
import { Identity } from "@semaphore-protocol/identity"
import { Group } from "@semaphore-protocol/group"
import { generateProof, unpackGroth16Proof } from "@semaphore-protocol/proof"
import { keccak256, toBeHex, concat, toUtf8Bytes, getBytes } from "ethers"
import { readFileSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import * as pkg from "@stellar/stellar-sdk"
const { rpc, TransactionBuilder, Operation, Keypair, Networks, authorizeEntry, xdr } = pkg

const AID = process.argv[2]
if (!/^C[A-Z0-9]{55}$/.test(AID)) throw new Error("bad AID: " + AID)
const PASSPHRASE = Networks.TESTNET
const SRC = Keypair.fromSecret(readFileSync("/tmp/feemeasure.secret", "utf8").trim())

const ART = join(tmpdir(), "snark-artifacts", "semaphore", "4.13.0")
const SNARK = { wasm: join(ART, "semaphore-2.wasm"), zkey: join(ART, "semaphore-2.zkey") }

// Same deterministic group used at deploy (live-account.mjs).
const member = new Identity("account-live-member-A")
const B = new Identity("account-live-member-B")
const C = new Identity("account-live-member-C")
const group = new Group([member.commitment, B.commitment, C.commitment])

const SEED_HEX = "33".repeat(32)
const scopeBig = BigInt(keccak256(concat([toUtf8Bytes(AID), getBytes("0x" + SEED_HEX)]))) // domain-bound
const be32 = (d) => BigInt(d).toString(16).padStart(64, "0")
const buf = (h) => Buffer.from(h, "hex")
const bv = (x) => xdr.ScVal.scvBytes(x)

async function sign(_pre, payload) {
    const message = BigInt("0x" + Buffer.from(payload).toString("hex"))
    const p = await generateProof(member, group, message, scopeBig, 2, SNARK)
    const u = unpackGroth16Proof(p.points)
    const sig = xdr.ScVal.scvMap([
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("a"), val: bv(buf(be32(u.pi_a[0]) + be32(u.pi_a[1]))) }),
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("b"), val: bv(buf(be32(u.pi_b[0][1]) + be32(u.pi_b[0][0]) + be32(u.pi_b[1][1]) + be32(u.pi_b[1][0]))) }),
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("c"), val: bv(buf(be32(u.pi_c[0]) + be32(u.pi_c[1]))) }),
        new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("nullifier"), val: bv(buf(be32(p.nullifier))) }),
    ])
    return { signatureScVal: sig }
}

const server = new rpc.Server("https://soroban-testnet.stellar.org")
const op = (auth) => Operation.invokeContractFunction({ contract: AID, function: "protected", args: [], auth })

let tx = new TransactionBuilder(await server.getAccount(SRC.publicKey()), { fee: "1000000", networkPassphrase: PASSPHRASE }).addOperation(op([])).setTimeout(120).build()
const sim = await server.simulateTransaction(tx)
if (rpc.Api.isSimulationError(sim)) throw new Error("sim: " + sim.error)
const validUntil = (await server.getLatestLedger()).sequence + 100
const signed = []
for (const e of sim.result.auth) signed.push(await authorizeEntry(e, sign, validUntil, PASSPHRASE))
let tx2 = new TransactionBuilder(await server.getAccount(SRC.publicKey()), { fee: "1000000", networkPassphrase: PASSPHRASE }).addOperation(op(signed)).setTimeout(120).build()
const sim2 = await server.simulateTransaction(tx2)
if (rpc.Api.isSimulationError(sim2)) throw new Error("sim2: " + sim2.error)
tx2 = rpc.assembleTransaction(tx2, sim2).build()
tx2.sign(SRC)
const res = await server.sendTransaction(tx2)
let g = await server.getTransaction(res.hash)
for (let i = 0; i < 40 && g.status === "NOT_FOUND"; i++) { await new Promise((r) => setTimeout(r, 2000)); g = await server.getTransaction(res.hash) }
console.log("CHECK_AUTH STATUS:", g.status, "| tx:", res.hash)
console.log("returnValue:", g.status === "SUCCESS" ? pkg.scValToNative(g.returnValue) : "(n/a)")
process.exit(g.status === "SUCCESS" ? 0 : 1)
