# Development

## Git hooks

We use [cargo-husky](https://crates.io/crates/cargo-husky) to manage our git hooks. The hooks are installed by running
`cargo test`.

```shell
# Install required cargo dependencies
# We use nextest to run our unit tests
cargo binstall cargo-nextest # or `cargo install cargo-nextest`
# Install the git hooks
cargo clean
cargo test
```

## Code Coverage

Code coverage stats are generated automatically in CI. To generate coverage stats locally, run the following:

```shell
# Install coverage dependencies
cargo binstall grcov
rustup component add llvm-tools
# If you have Nix on you system, you can install the `genhtml` command using the nix package.
# Todo: other methods of installing `genhtml`
nix-env -iA nixpkgs.lcov
# Build + run tests with coverage
cargo llvm-cov --no-report nextest --all-features 
# Generate and open an HTML coverage report
cargo llvm-cov report --lcov --output-path ./target/llvm-cov-target/debug/lcov.info
genhtml -o ./target/llvm-cov-target/debug/coverage/ --show-details --highlight --ignore-errors source --legend ./target/llvm-cov-target/debug/lcov.info
open target/llvm-cov-target/debug/coverage/index.html
```

# Releases

Releases are created and published to crates.io using [release-plz](https://github.com/MarcoIeni/release-plz) via
our [Release PR][ReleasePRLink] and [Release][ReleaseLink] workflows. The [Release PR][ReleasePRLink] workflow runs when
the [Feature Powerset][FeaturePowersetLink] workflow completes successfully. The [Feature Powerset][FeaturePowersetLink]
workflow is scheduled to run once a week, so release PR will also be created once a week. If an unscheduled release
needs to be created, the [Feature Powerset][FeaturePowersetLink] workflow can also be triggered manually, and
the [Release PR][ReleasePRLink] workflow will run once the [Feature Powerset][FeaturePowersetLink] workflow completes.
Once a release PR is merged to `main`, the [Release][ReleaseLink] workflow will publish the release.

Note that we use the default `GITHUB_TOKEN` to create the release PRs. This means that the PR checks [will not
run automatically](https://docs.github.com/en/actions/security-guides/automatic-token-authentication#using-the-github_token-in-a-workflow).
To get the checks to run, simply close and re-open the release PR.

[ReleaseLink]: https://github.com/roadster-rs/roadster/blob/main/.github/workflows/release.yml

[ReleasePRLink]: https://github.com/roadster-rs/roadster/blob/main/.github/workflows/release_pr.yml

[FeaturePowersetLink]: https://github.com/roadster-rs/roadster/blob/main/.github/workflows/feature_powerset.yml
