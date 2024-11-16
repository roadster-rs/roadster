# List the available commands.
help:
  @just --list --justfile {{justfile()}}

# Run all of our unit tests.
test:
    cargo nextest run --all-features --no-fail-fast

test-doc:
    cargo test --doc --all-features --no-fail-fast

test-examples:
    for dir in ./examples/*/; do cd $dir && pwd && cargo test --all-features --no-fail-fast && cd ../.. && pwd; done

test-private:
    for dir in ./private/*/; do cd $dir && pwd && cargo test --all-features --no-fail-fast && cd ../.. && pwd; done

# Run all of our unit tests.
test-unit: test test-doc

test-book:
    mdbook test book

test-book-examples:
    for dir in ./book/examples/*/; do cd $dir && pwd && cargo test --all-features --no-fail-fast && cd ../../.. && pwd; done

test-book-all: test-book test-book-examples

# Run all of our tests
test-all: test-unit test-book-all

# Run all of our unit tests whenever files in the repo change.
test-watch:
    cargo watch -s 'just test'

insta:
    cargo insta review --workspace

serve-book:
    mdbook serve book

# Install dependencies required to generate code coverage.
coverage-dependencies:
    cargo binstall cargo-llvm-cov
    # Nightly is required for doctests, as well as the `coverage(off)` attribute
    rustup toolchain install nightly
    rustup component add llvm-tools
    nix-env -iA nixpkgs.lcov

# Remove previously generated code coverage data in order to get an accurate report.
coverage-clean:
    cargo +nightly llvm-cov clean

# Run tests with coverage.
coverage-tests: coverage-clean
    cargo +nightly llvm-cov --no-report nextest --all-features
    cargo +nightly llvm-cov --no-report --doc --all-features

# Run tests with coverage and generate an html report.
coverage: coverage-tests
    # Generate and open an HTML coverage report
    cargo +nightly llvm-cov report --lcov --output-path ./target/llvm-cov-target/debug/lcov.info
    genhtml -o ./target/llvm-cov-target/debug/coverage/ --show-details --highlight --ignore-errors source --legend ./target/llvm-cov-target/debug/lcov.info

open_cmd := if os() == "macos" { "open" } else { "xdg-open" }

# Run tests with coverage and open the generated HTML report.
coverage-open: coverage
    {{ open_cmd }} target/llvm-cov-target/debug/coverage/index.html

alias fmt := format
# Format the project
format:
    cargo fmt

check-fmt:
    cargo fmt --all --check

pre-commit: check-fmt
pre-push: check-fmt

check-no-features:
    cargo nextest run --no-default-features --no-fail-fast
    # Nextest doesn't support doc tests, run those separately
    cargo test --doc --no-default-features --no-fail-fast
    cargo check --no-default-features
    cargo clippy --all-targets --no-default-features -- -D warnings

check-default-features:
    # With default features
    cargo nextest run --no-fail-fast
    # Nextest doesn't support doc tests, run those separately
    cargo test --doc --no-fail-fast
    cargo check
    cargo clippy --all-targets -- -D warnings

check-all-features:
    # With all features
    cargo nextest run --all-features --no-fail-fast
    # Nextest doesn't support doc tests, run those separately
    cargo test --doc --all-features --no-fail-fast
    cargo check --all-features
    cargo clippy --all-targets --all-features -- -D warnings

check-docs:
    RUSTDOCFLAGS="-D rustdoc::all -A rustdoc::private_intra_doc_links" cargo doc --all-features --no-deps

check-msrv:
    cargo minimal-versions check --direct --all-features --no-dev-deps

# Run a suite of checks. These checks are fairly comprehensive and will catch most issues. However, they are still less than what is run in CI.
check: check-fmt check-no-features check-default-features check-all-features check-docs check-msrv

# Check if the Codecov config is valid
validate-codecov-config:
    curl -X POST --data-binary @codecov.yml https://codecov.io/validate

# Initialize a new installation of the repo (e.g., install deps)
init:
    cargo binstall cargo-nextest cargo-llvm-cov sea-orm-cli cargo-insta cargo-minimal-versions cargo-hack mdbook cargo-deny
