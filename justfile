default:
    @just --list

build:
    cargo build --workspace

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

unit:
    cargo test --workspace --all-targets

mutants:
    cargo mutants --no-shuffle

deny:
    cargo deny check

test: unit mutants

check: fmt-check lint test deny

hooks:
    pre-commit install --install-hooks
    pre-commit install --hook-type commit-msg
