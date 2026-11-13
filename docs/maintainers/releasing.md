# Release procedure

Use this procedure for a reviewed release candidate. The [package README](../../README.md)
describes the supported scope; [API stability](../reference/api-stability.md)
defines the compatibility contract. This procedure is repository-only.

## Candidate review

- Review the full diff and freeze a clean release commit, preserving other work.
- Confirm version, changelog, installation instructions, licenses and Rust minimum.
  Prepare final release wording in the reviewed candidate; do not ship an obsolete
  unpublished banner.
- Review the provider coverage and field claims for this release, including the
  [documented Bulgarian limitations](../reference/bulgarian-registry.md#coverage-limitations).
  Synthetic success and address-only reviews do not establish broader live coverage.
- Verify crates.io name availability and publisher ownership/access.

## Verification

Run all [contributor checks](../../CONTRIBUTING.md#setup-and-checks) and obtain passing
minimum-version and stable-Rust Linux CI results for the exact candidate commit.
Run the separate local corpus regressions when the retained observations are
available, and record that evidence separately from public CI. Check both public
source history and the crate archive: collected corpora, identifier queues and
provenance must remain local-only. A local candidate and its
local checks must not be reported as passing remote CI.

After `cargo package --locked`, run the archive's tests. With Cargo's default target
location, substitute the candidate version below; when using `CARGO_TARGET_DIR`,
use that directory's `package/` subdirectory instead:

```sh
CRATE_VERSION='X.Y.Z'
cargo test --locked --all-targets --manifest-path "target/package/eu-company-lookup-$CRATE_VERSION/Cargo.toml"
cargo test --locked --doc --manifest-path "target/package/eu-company-lookup-$CRATE_VERSION/Cargo.toml"
cargo publish --locked --dry-run
```

Audit the locked dependencies against the current RustSec advisory database using
`cargo audit`; resolve vulnerabilities and record any informational warnings.
Compile a separate consumer against the extracted package with a fresh lockfile
to check dependency resolution independently of this repository.

Review the generated archive and publish dry-run output. Record the commit, archive
checksum, toolchain and check results in bounded release evidence, not as a running
history in README. Do not use `--no-verify`. `--allow-dirty` is only for local
preparation and does not satisfy the clean-commit release gate.

The crate contains code, synthetic tests, examples, consumer documentation and
license notices. It excludes developer queues, collected response corpora, source
metadata, tools, agent/contributor instructions, release checklists and local reports.

## First-time publisher setup

GitHub hosts this repository; [crates.io](https://crates.io/) distributes the Rust
package. They are separate services. Follow the [Cargo publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html):

1. Sign into crates.io with your GitHub account.
2. Add and verify your email in [account settings](https://crates.io/settings/profile).
3. Create a publishing token in [API tokens](https://crates.io/settings/tokens).
4. Run `cargo login` in the environment that will publish, then paste the token at
   its prompt. Do not put it in chat, shell arguments, source files or CI logs.
5. After candidate approval, publish from the clean reviewed commit with
   `cargo publish --locked`. The initial publication registers the crate name;
   a successful dry run does not reserve it.

The account that first publishes owns the crate. GitHub repository access does
not automatically grant crates.io ownership. Additional maintainers or a GitHub
organization team can be added later with `cargo owner`; keep a named account as
an owner. docs.rs builds hosted API documentation after publication.

If using a container, `cargo login` must use the same persistent `CARGO_HOME` as the
publishing command. Authenticating the host alone does not authenticate a container.

## Approval and publication

Present the exact candidate, validation results and outstanding limitations for the
user's explicit approval. Preparation and successful dry runs do not authorize upload.

After approval, publish that candidate, tag the same commit `v<version>`, and create its
release notes. Verify fresh installation from crates.io and docs.rs output. Ensure
the published README describes the actual release state. If publication outcome is
uncertain, inspect the registry before retrying; never overwrite or move a release tag.

Use [Cargo's publishing reference](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)
for command behavior. Future API compatibility reviews should compare against the
latest approved release. No automatic publication workflow is configured.
