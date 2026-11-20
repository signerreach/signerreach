#!/usr/bin/env bash
# Reads three public mainnet accounts with `signerreach fetch`, then runs a
# state-only `signerreach check` (an empty plan) on each, under two declared
# availability assumptions. Read-only: it signs and submits nothing.
#
#   bash run.sh <path-to-signerreach-binary> <output-directory>
#
# The same script is meant to be run again after the network changes protocol, so
# the two runs can be compared. Nothing in it depends on the date or the protocol.
set -u

BIN=${1:?usage: run.sh <signerreach-binary> <output-directory>}
OUT=${2:?usage: run.sh <signerreach-binary> <output-directory>}

# Public mainnet accounts, chosen for variety (a multisig with its master key
# disabled, an account with eight signers, a single-key account). They are listed
# by address only; the ledger is the source for everything else.
ACCOUNTS="
GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN
GBVOL67TMUQBGL4TZYNMY3ZQ5WGQYFPFD5VJRWXR72VA33VFNL225PL5
GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX
"
HORIZON=https://horizon.stellar.org

mkdir -p "$OUT/inputs" "$OUT/snapshots" "$OUT/reports" "$OUT/horizon"

# Inputs. The plan is empty, so no change to any account is evaluated.
printf '%s\n' '{ "schema": "signerreach.plan/1", "transactions": [] }' > "$OUT/inputs/plan-empty.json"
# Assumption A (hypothetical): every signer of the account can sign.
printf '%s\n' '{ "schema": "signerreach.availability/1", "default": "available" }' > "$OUT/inputs/availability-all-available.json"
# Assumption B: nothing is declared, so the tool must not assume anything.
printf '%s\n' '{ "schema": "signerreach.availability/1", "default": "unknown" }' > "$OUT/inputs/availability-unknown.json"

{
  echo "binary_version: $("$BIN" --version 2>&1 | head -1)"
  echo "run_started_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "$OUT/run-info.txt"

curl -s -m 30 "$HORIZON/" > "$OUT/horizon/root-before.json"

printf 'account\tstep\texit_code\n' > "$OUT/exit-codes.tsv"
for A in $ACCOUNTS; do
  "$BIN" fetch --network pubnet --account "$A" > "$OUT/snapshots/$A.json" 2> "$OUT/snapshots/$A.stderr"
  printf '%s\t%s\t%s\n' "$A" fetch $? >> "$OUT/exit-codes.tsv"
  curl -s -m 30 "$HORIZON/accounts/$A" > "$OUT/horizon/$A.account.json"

  for V in all-available unknown; do
    for F in text json; do
      EXT=txt; [ "$F" = json ] && EXT=json
      "$BIN" check --snapshot "$OUT/snapshots/$A.json" \
        --plan "$OUT/inputs/plan-empty.json" \
        --availability "$OUT/inputs/availability-$V.json" \
        --format "$F" > "$OUT/reports/$A.$V.$EXT" 2> "$OUT/reports/$A.$V.$EXT.stderr"
      printf '%s\tcheck-%s-%s\t%s\n' "$A" "$V" "$F" $? >> "$OUT/exit-codes.tsv"
    done
  done
done

curl -s -m 30 "$HORIZON/" > "$OUT/horizon/root-after.json"
echo "run_finished_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$OUT/run-info.txt"

# Checksums of everything produced, so a later edit is visible.
( cd "$OUT" && find . -type f ! -name SHA256SUMS | LC_ALL=C sort | xargs sha256sum > SHA256SUMS )
echo "done: $OUT"
