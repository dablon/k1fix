set shell := ["bash", "-cu"]

default:
    @just --list

dev:
    docker compose run --rm dev

test:
    docker compose run --rm test

lint:
    docker compose run --rm lint

cov:
    docker compose run --rm coverage

e2e:
    docker compose run --rm e2e

build:
    docker compose run --rm dev cargo build --release --bin k1fix

run *ARGS:
    docker compose run --rm --entrypoint k1fix runtime {{ARGS}}

fixtures:
    cargo run --example gen_fixtures

# Local helpers (Rust on host)
host-test:
    cargo test
    cargo test --tests

host-lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings

host-cov:
    cargo llvm-cov --all-features \
      --ignore-filename-regex '(main\.rs|examples/)' \
      --fail-under-lines 80 \
      --html --output-dir coverage

host-release:
    cargo build --release --bin k1fix
