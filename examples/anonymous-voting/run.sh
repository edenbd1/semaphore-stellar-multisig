#!/usr/bin/env bash
# Flagship demo: run a full anonymous vote on Stellar testnet, powered by Semaphore.
# A committee votes anonymously on a proposal; at threshold it executes. You can see that
# N distinct members approved, never which ones. Usage: ./run.sh [identity]
set -euo pipefail
cd "$(dirname "$0")"
IDENTITY="${1:-semaphore-dev}"
stellar keys generate "$IDENTITY" --network testnet 2>/dev/null || true
stellar keys fund "$IDENTITY" --network testnet 2>/dev/null || true

[ -d node_modules ] || npm install --silent
echo "== generating the committee and anonymous votes =="
node gen.mjs

echo "== building the voting contract (ConfidentialDao) =="
( cd ../.. && stellar contract build >/dev/null )
WASM=../../target/wasm32v1-none/release/semaphore_dao.wasm

val(){ node -e "console.log(require('./voting-args.json').$1)"; }
ROOT=$(val root); TH=$(val threshold); ACTION=$(val action); SCOPE=$(val scope_seed)
VK=$(node -e "console.log(JSON.stringify(require('./voting-args.json').vk))")
N=$(node -e "console.log(require('./voting-args.json').votes.length)")

echo "== deploying a fresh voting contract on testnet =="
stellar contract deploy --wasm "$WASM" --source "$IDENTITY" --network testnet -- \
  --root "$ROOT" --depth 2 --threshold "$TH" --vk "$VK" > /tmp/vote_deploy.out 2>&1
DID=$(grep -oE 'C[A-Z0-9]{55}' /tmp/vote_deploy.out | tail -1)
echo "voting contract: $DID"
inv(){ stellar contract invoke --id "$DID" --source "$IDENTITY" --network testnet --send=yes -- "$@"; }

echo "== opening the proposal =="
inv create_proposal --action "$ACTION" --scope_seed "$SCOPE" >/dev/null
PID=0  # first proposal on a fresh contract

echo "== casting votes until the threshold is reached =="
for i in $(seq 0 $((TH-1))); do
  ENTRY=$(node -e "console.log(JSON.stringify(require('./voting-args.json').votes[$i]))")
  COUNT=$(inv approve --proposal_id "$PID" --entry "$ENTRY" | tail -1)
  echo "   anonymous vote $((i+1)) accepted, approvals = $COUNT"
done

echo "== executing (threshold reached) =="
inv execute --proposal_id "$PID" >/dev/null
echo "== final proposal state =="
stellar contract invoke --id "$DID" --source "$IDENTITY" --network testnet -- proposal --proposal_id "$PID"
echo ""
echo "Done. The chain shows $N distinct members approved and the proposal executed —"
echo "without revealing which members voted. That is Semaphore on Stellar."
