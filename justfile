default:
  @just --list --justfile {{justfile()}}

test:
    cargo nextest run --all-features

test-watch:
    cargo watch -x 'nextest run --all-features'

coverage-dependencies:
    cargo binstall cargo-llvm-cov
    # Nightly is required for doctests, as well as the `coverage(off)` attribute
    rustup toolchain install nightly
    rustup component add llvm-tools
    nix-env -iA nixpkgs.lcov

coverage-clean:
    cargo +nightly llvm-cov clean

coverage: coverage-clean
    cargo +nightly llvm-cov --no-report nextest --all-features
    cargo +nightly llvm-cov --no-report --doc --all-features
    # Generate and open an HTML coverage report
    cargo +nightly llvm-cov report --lcov --output-path ./target/llvm-cov-target/debug/lcov.info
    genhtml -o ./target/llvm-cov-target/debug/coverage/ --show-details --highlight --ignore-errors source --legend ./target/llvm-cov-target/debug/lcov.info

coverage-open: coverage
    open target/llvm-cov-target/debug/coverage/index.html

update:
    cargo upgrade

