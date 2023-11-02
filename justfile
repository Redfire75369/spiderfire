set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

default:
    @just --list

build *args:
  cargo build --features debugmozjs {{args}}

build-release *args:
  cargo build --release {{args}}

run *args:
  cargo run --features debugmozjs {{args}}

run-release *args:
  cargo run --release {{args}}

test *args:
  cargo test --features debugmozjs --locked --no-fail-fast {{args}}

test-release *args:
  cargo test --release --locked --no-fail-fast {{args}}

lint:
  cargo fmt --check --all
  cargo clippy --all-targets --locked -- -D warnings
