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
coverage: coverage-clean
    cargo +nightly llvm-cov --no-report nextest --all-features
    cargo +nightly llvm-cov --no-report --doc --all-features
    # Generate and open an HTML coverage report
    cargo +nightly llvm-cov report --lcov --output-path ./target/llvm-cov-target/debug/lcov.info
    genhtml -o ./target/llvm-cov-target/debug/coverage/ --show-details --highlight --ignore-errors source --legend ./target/llvm-cov-target/debug/lcov.info

# Run tests with coverage and open the generated HTML report.
coverage-open: coverage
    open target/llvm-cov-target/debug/coverage/index.html

alias fmt := format
# Format the project
format:
    cargo fmt

# Run a suite of checks. These checks are fairly comprehensive and will catch most issues. However, they are still less than what is run in CI.
check:
    .cargo-husky/hooks/pre-push

# Check if the Codecov config is valid
validate-codecov-config:
    curl -X POST --data-binary @codecov.yml https://codecov.io/validate

# Initialize a new installation of the repo (e.g., install deps)
init:
    cargo binstall cargo-nextest cargo-llvm-cov sea-orm-cli cargo-insta cargo-minimal-versions cargo-hack mdbook
