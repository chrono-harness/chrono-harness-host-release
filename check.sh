#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p target
scratch=$(mktemp -d "$(pwd -P)/target/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export TMPDIR="$scratch"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/host-release/Cargo.toml
product=crates/host-release/Cargo.toml
cargo build --locked --message-format=json-render-diagnostics --manifest-path "$product" > "$scratch/build.jsonl"
cargo fmt --check --manifest-path crates/host-release-tests/Cargo.toml
cargo test --locked --manifest-path crates/host-release-tests/Cargo.toml
coverage_failure() {
  echo "E_VERUS_COVERAGE: $*" >&2
  exit 1
}

prove() {
  chrono-host-bootstrap run-compiler-proof "$scratch" "$1" "${VERUS:-verus}" "$2" ||
    coverage_failure "$1: verifier execution failed"
}

admit_proof() {
  local operation=$1
  shift
  chrono-host-bootstrap collect-compiler-proof "$scratch" "$operation" "${VERUS:-verus}" \
    0.2026.04.24.f8e1704 f8e17045c4ac17a02e566b8997070f2eecc255be "$@" ||
    coverage_failure "$operation: required proof execution was not admitted"
}

# Read the exact library selected by Cargo in the build of the tested executable.
metadata=$(jq -ers --arg manifest "$(pwd -P)/$product" '
  [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
                 and .target.kind == ["lib"]) | .filenames[] | select(endswith(".rmeta"))]
  | if length == 1 then .[0] else error("expected the product library") end
' "$scratch/build.jsonl") || coverage_failure "native library was not identified"
filename=${metadata##*/}
stem=${filename#lib}
stem=${metadata%/*}/${stem%.rmeta}
chrono-host-bootstrap check-compiler-sources native "$stem" \
  src/lib.rs src/verified_recipe.rs src/verified_delivery.rs src/recipe.rs \
  src/frontend.rs src/execution.rs src/receipt.rs src/preview.rs \
  src/staging.rs src/delivery.rs src/publication.rs ||
  coverage_failure "native library differs from required sources"

# The CLI root participates in the proof and is compiled as Cargo's binary target.
executable=$(jq -ers --arg manifest "$(pwd -P)/$product" --arg source "$(pwd -P)/crates/host-release/src/main.rs" '
  [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
                 and .target.kind == ["bin"] and .target.src_path == $source) | .executable]
  | if length == 1 and .[0] != null then .[0] else error("expected the release executable") end
' "$scratch/build.jsonl") || coverage_failure "native release entry was not identified"
test -x "$executable" || coverage_failure "native release executable is missing"

src=crates/host-release/src
tests=crates/host-release-tests/tests
included="$tests/../../host-release/src"

prove verify.release-recipe "$src/verified_recipe.rs"
prove verify.release-recipe-tests "$tests/verified_recipe.rs"

admit_proof verify.release-recipe "$src/verified_recipe.rs" "$src/main.rs" \
  "$src/execution.rs" "$src/frontend.rs" \
  "$src/receipt.rs" "$src/recipe.rs" \
  "$src/preview.rs" "$src/staging.rs" \
  "$src/delivery.rs" "$src/publication.rs" \
  "$src/verified_delivery.rs"
admit_proof verify.release-recipe-tests "$tests/verified_recipe.rs" "$included/verified_recipe.rs" \
  "$included/main.rs" "$included/execution.rs" \
  "$included/frontend.rs" "$included/receipt.rs" \
  "$included/recipe.rs" "$included/preview.rs" \
  "$included/staging.rs" "$included/delivery.rs" \
  "$included/publication.rs" "$included/verified_delivery.rs" \
  "$tests/proof_support/host-release/publication.rs" "$tests/proof_support/host-release/recipe.rs" \
  "$tests/proof_support/host-release/support/execution.rs" "$tests/proof_support/host-release/support/failure_case.rs" \
  "$tests/proof_support/host-release/support/frontend.rs" "$tests/proof_support/host-release/staging.rs" \
  "$tests/proof_support/host-release/support/release_tool.rs"
