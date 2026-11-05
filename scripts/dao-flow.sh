#!/usr/bin/env bash
# End-to-end confidential DAO flow on testnet: deploy -> create proposal -> two members
# approve (one proof per tx) -> execute. Usage: scripts/dao-flow.sh [identity]
#
# NOTE: this fixture-based flow predates domain separation and no longer works as-is — the scope
# is now derived on-chain from the DEPLOYED contract address, so the approval proofs must be
# generated AFTER deploy (bound to that address). Use the deploy-then-prove helper
# tools/live-dao.mjs instead (see TESTNET-LIVE-STATUS in the plan notes). Kept for reference.
set -euo pipefail
IDENTITY="${1:-semaphore-dev}"
cd "$(dirname "$0")/.."
stellar keys generate "$IDENTITY" --network testnet 2>/dev/null || true
stellar keys fund "$IDENTITY" --network testnet 2>/dev/null || true

stellar contract build
A=$(node tools/emit-dao-args.mjs)
val(){ echo "$A" | node -e "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>{const o=JSON.parse(d);console.log($1)})"; }
VK=$(val 'JSON.stringify(o.vk)'); ROOT=$(val 'o.root'); PAY=$(val 'o.payload')
EA=$(val 'JSON.stringify(o.entry_a)'); EB=$(val 'JSON.stringify(o.entry_b)')

WASM=target/wasm32v1-none/release/semaphore_dao.wasm
DID=$(stellar contract deploy --wasm "$WASM" --source "$IDENTITY" --network testnet -- \
  --root "$ROOT" --depth 2 --threshold 2 --vk "$VK" | grep -oE 'C[A-Z0-9]{55}' | tail -1)
echo "DAO_ID=$DID"
inv(){ stellar contract invoke --id "$DID" --source "$IDENTITY" --network testnet --send=yes -- "$@"; }
PID=$(inv create_proposal --action "$PAY" --scope_seed "$PAY" | tail -1)
inv approve --proposal_id "$PID" --entry "$EA"
inv approve --proposal_id "$PID" --entry "$EB"
inv execute --proposal_id "$PID"
stellar contract invoke --id "$DID" --source "$IDENTITY" --network testnet -- proposal --proposal_id "$PID"
