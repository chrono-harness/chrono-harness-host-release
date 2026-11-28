#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
if [[ $# -eq 0 ]]; then
  /bin/sh .chrono-harness/ci/install.sh
  export PATH="$PWD/.chrono-harness/bin:$PATH"
  export CHRONO_CHECK_SOURCE=${CHRONO_CHECK_SOURCE:-local}
  exec .chrono-harness/bin/chrono-harness check
fi
if [[ $# -ne 1 || $1 != --native ]]; then
  echo 'usage: ./check.sh [--native]' >&2
  exit 2
fi
scratch_root=$(pwd -P)/.chrono-harness/state/native-check
mkdir -p "$scratch_root"
scratch=$(mktemp -d "$scratch_root/check.XXXXXXXX")
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

# Read the exact library selected by Cargo in the build of the tested executable.
stem=$(chrono-host-bootstrap select-native-cargo-library "$(pwd -P)/$product" "$scratch/build.jsonl") ||
  coverage_failure "native library was not identified"
chrono-host-bootstrap check-compiler-sources native "$stem" \
  src/lib.rs src/bootstrap.rs src/files.rs src/verified_recipe.rs src/codec.rs src/identity.rs src/verified_delivery.rs src/version_order.rs src/recipe.rs src/registrations.rs \
  src/local.rs src/frontend.rs src/evidence.rs src/execution.rs src/receipt.rs src/preview.rs \
  src/process.rs src/transport.rs src/staging.rs src/delivery.rs src/publication.rs src/template_run.rs src/roadmap.rs ||
  coverage_failure "native library differs from required sources"

# The CLI root participates in the proof and is compiled as Cargo's binary target.
executable=$(chrono-harness compiler-output-v1 select-cargo-bin \
  "$(pwd -P)/$product" "$(pwd -P)/crates/host-release/src/main.rs" < "$scratch/build.jsonl") ||
  coverage_failure "native release entry was not identified"
test -x "$executable" || coverage_failure "native release executable is missing"

src=crates/host-release/src
tests=crates/host-release-tests/tests
included="$tests/../../host-release/src"

chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/release-proofs/verify.release-recipe.rlib "$src/verified_recipe.rs" "$src/bootstrap.rs" "$src/codec.rs" "$src/main.rs" "$src/committed.rs" \
  "$src/evidence.rs" "$src/execution.rs" "$src/files.rs" "$src/frontend.rs" "$src/local.rs" \
  "$src/receipt.rs" "$src/recipe.rs" \
  "$src/preview.rs" "$src/registrations.rs" "$src/staging.rs" "$src/transport.rs" \
  "$src/delivery.rs" "$src/publication.rs" \
  "$src/observed.rs" "$src/process.rs" "$src/template_run.rs" "$src/version_order.rs" "$src/roadmap.rs" "$src/verified_delivery.rs" "$src/identity.rs"
chrono-host-bootstrap check-compiler-sources proof .chrono-harness/state/release-proofs/verify.release-recipe-tests.rlib "$tests/verified_recipe.rs" "$included/verified_recipe.rs" "$included/bootstrap.rs" "$included/codec.rs" \
  "$included/main.rs" "$included/committed.rs" "$included/evidence.rs" "$included/execution.rs" \
  "$included/files.rs" "$included/frontend.rs" "$included/local.rs" "$included/receipt.rs" \
  "$included/recipe.rs" "$included/preview.rs" "$included/registrations.rs" \
  "$included/staging.rs" "$included/transport.rs" "$included/delivery.rs" \
  "$included/publication.rs" "$included/observed.rs" "$included/process.rs" "$included/template_run.rs" "$included/version_order.rs" "$included/roadmap.rs" "$included/verified_delivery.rs" "$included/identity.rs" \
  "$tests/originals.rs" "$tests/query_tool.rs" \
  "$tests/proof_support/host-release/publication.rs" "$tests/proof_support/host-release/recipe.rs" \
  "$tests/proof_support/host-release/support/execution.rs" "$tests/proof_support/host-release/support/failure_case.rs" \
  "$tests/proof_support/host-release/support/frontend.rs" "$tests/proof_support/host-release/staging.rs" \
  "$tests/proof_support/host-release/support/release_tool.rs" "$tests/proof_support/host-release/support/../../../../../../check.sh"
