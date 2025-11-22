# Justfile for dbctl

# Default recipe to display help
default:
    @just --list

# Build the project in debug mode
build:
    cargo build

# Run all tests (including unit and integration tests)
test:
    cargo test -- --test-threads=1

# Format all code
fmt:
    cargo fmt

# Run clippy for linting
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run format and lint checks
check: fmt clippy

# Install dbctl to ~/.cargo/bin
install:
    cargo install --path .
