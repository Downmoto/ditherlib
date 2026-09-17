# Releasing ditherlib

Releases to crates.io are manual. Merging a package version change into
`master` triggers `.github/workflows/release.yml`, which creates the matching
Git tag and GitHub release.

## Prepare the release

1. Start a `chore/release-v<version>` branch from an up-to-date `master`.
2. Update the version in `Cargo.toml` and `Cargo.lock`.
3. Add the release notes to `CHANGELOG.md`.
4. Update the README dependency example for the new release line.
5. Commit these changes together with a Conventional Commit such as
   `chore: release 1.0.0`.

Run the complete release gate from a clean working tree:

```console
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --no-default-features
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo package --locked
cargo package --list --locked
cargo publish --dry-run --locked
git diff --check
```

Inspect the package file list for generated files, local assets, fixtures, and
other content that should not be published. Open a pull request only after all
checks pass.

## Merge and publish

1. Merge the release pull request after CI passes.
2. Wait for CI on `master` and the GitHub Release workflow to succeed.
3. Update the local `master` branch and confirm the working tree is clean.
4. Confirm that the generated Git tag and GitHub release use the package
   version and point to the release commit.
5. Run `cargo publish --dry-run --locked` once more from `master`.
6. Run `cargo publish --locked` when ready to make the release public.

Publishing requires an authenticated crates.io account with ownership of the
crate. Keep API tokens in Cargo's credential storage or an environment secret;
never add them to the repository.

## Verify the release

- Confirm the new version appears on crates.io with the expected metadata and
  README.
- Confirm docs.rs finishes building the same version without warnings.
- Confirm the GitHub tag and release point to the release commit.
- Create a temporary crate and resolve the published version from crates.io.

Crates.io releases cannot be replaced. If publication succeeds and a defect is
found, prepare a patch release. If publication fails before crates.io accepts
the package, diagnose the error and repeat the dry run before retrying.
