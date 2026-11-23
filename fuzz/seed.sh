#!/usr/bin/env bash
# Builds the starting corpus for the fuzz targets from the repository's own
# fixtures and recorded Horizon responses, so the fuzzer starts from inputs the
# tool really reads instead of from nothing.
#
#   bash fuzz/seed.sh       writes fuzz/corpus/<target>/...
#
# The corpus directory is not committed; it is rebuilt from the files named here.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
corpus="$root/fuzz/corpus"

rm -rf "$corpus"
mkdir -p "$corpus/parse_documents" "$corpus/evaluate_report" \
         "$corpus/horizon_mapping" "$corpus/network_text"

for dir in "$root"/fixtures/fx-*/; do
  name=$(basename "$dir")
  for document in snapshot plan availability; do
    cp "$dir/$document.json" "$corpus/parse_documents/$name-$document.json"
  done
  # snapshot NUL plan NUL availability
  {
    cat "$dir/snapshot.json"
    printf '\0'
    cat "$dir/plan.json"
    printf '\0'
    cat "$dir/availability.json"
  } > "$corpus/evaluate_report/$name"
done

# Ten bytes choose the network, the protocol, the ledger and the reserve data,
# then the record.
# 00 00 = testnet, protocol 29;  01 01 = pubnet, protocol 30;  ledger bytes are
# large enough to be at or after every recorded response. Byte 6 = 01 asks for
# `reserve` data, and the next three bytes (40 4b 4c) are a base reserve of
# 5,000,000 stroops.
for record in "$root"/crates/signerreach-stellar/tests/data/horizon-*.json; do
  name=$(basename "$record" .json)
  { printf '\000\000\377\377\377\177\000\000\000\000'; cat "$record"; } \
    > "$corpus/horizon_mapping/$name-testnet29"
  { printf '\001\001\377\377\377\177\000\000\000\000'; cat "$record"; } \
    > "$corpus/horizon_mapping/$name-pubnet30"
  { printf '\000\000\377\377\377\177\001\100\113\114'; cat "$record"; } \
    > "$corpus/horizon_mapping/$name-testnet29-reserve"
  { printf '\001\001\377\377\377\177\001\100\113\114'; cat "$record"; } \
    > "$corpus/horizon_mapping/$name-pubnet30-reserve"
done

# address NUL Date header NUL server root (also read as a ledger record) NUL
# Latest-Ledger header
printf '%s\0%s\0%s\0%s' \
  'https://horizon-testnet.stellar.org' \
  'Sat, 17 Oct 2026 08:17:27 GMT' \
  '{"sequence":5240432,"protocol_version":30,"base_fee_in_stroops":100,"base_reserve_in_stroops":5000000}' \
  '5240432' > "$corpus/network_text/ledger"
printf '%s\0%s\0%s\0%s' \
  'https://horizon-testnet.stellar.org' \
  'Sat, 17 Oct 2026 08:17:27 GMT' \
  '{"network_passphrase":"Test SDF Network ; September 2015","current_protocol_version":30}' \
  '5240432' > "$corpus/network_text/testnet"
printf '%s\0%s\0%s\0%s' \
  'http://127.0.0.1:8000/' \
  'Mon, 29 Feb 2027 23:59:59 GMT' \
  '{"network_passphrase":"Public Global Stellar Network ; September 2015","current_protocol_version":29}' \
  '1' > "$corpus/network_text/local"
printf '%s\0%s\0%s\0%s' \
  'http://[::1]:8000' \
  'Thu, 01 Jan 1970 00:00:00 GMT' \
  '{}' \
  '18446744073709551615' > "$corpus/network_text/edges"

echo "seeded: $(find "$corpus" -type f | wc -l) files in $corpus"
