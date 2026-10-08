---
type: operations-delivery
title: CI Quality Gates, Dependency Audit, and Release
description: The four GitHub Actions workflows that gate changes, audit dependencies on a schedule, publish multi-platform binaries, and deploy the documentation site.
tags: [ci, github-actions, release, supply-chain, operations]
verified:
  - by: openwiki/0.7.1
    at: 2026-10-08T08:38:59.748Z
sources:
  - id: openwiki-source-207fa9d9092efa3fe808da7a
    resource: repo://.github/workflows/audit.yml
  - id: openwiki-source-7339406115330a5d3e8eea08
    resource: repo://.github/workflows/openwiki-page.yml
  - id: openwiki-source-c1a55afad9a61e65437aa93e
    resource: repo://.github/workflows/pr-gate.yml
  - id: openwiki-source-4d1d392666be6dfdd7a91a2e
    resource: repo://.github/workflows/release.yml
  - id: openwiki-source-845ef3fba7d5519c73daee5e
    resource: repo://deny.toml
generated: { by: "opencode", at: "2026-10-08T08:38:59.748Z" }
---

# CI Quality Gates, Dependency Audit, and Release

Four workflows cover the repository's non-runtime concerns. Three of them (PR gate, dependency audit, release) run on completely different triggers, which is the main thing to understand when reasoning about what protects a given change.

## PR gate — `pr-gate.yml`

Runs on pull requests targeting `main` and on pushes to `main`, with `cancel-in-progress` concurrency per ref (repo://.github/workflows/pr-gate.yml#L3-L16).

A `dorny/paths-filter` job first decides whether Rust is involved at all: `src/**`, `Cargo.toml`, `Cargo.lock`, `.rustfmt.toml`, any `rust-toolchain*` file, and the gate workflow itself. Only when that filter reports `true` does the three-OS matrix (Linux, macOS, Windows) run, so a docs-only change does not pay for three Rust toolchains (repo://.github/workflows/pr-gate.yml#L18-L52).

Each matrix leg installs `stable` with `clippy` and `rustfmt` and runs six checks in order: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo machete` for unused dependencies, `cargo build --release --locked`, `cargo test --all-features --locked`, and `cargo doc --no-deps --all-features` (repo://.github/workflows/pr-gate.yml#L64-L86). Two properties are worth stating plainly: `--locked` means `Cargo.lock` must already satisfy the build, so a lockfile update is part of the change under test; and `-D warnings` makes clippy findings build-breaking rather than advisory.

Two further jobs accompany the matrix. `lint-workflows` runs `actionlint` over the workflow files themselves, so a malformed workflow is caught rather than silently skipped. `sonarqube` `needs: quality-gate` and produces LCOV coverage via `cargo llvm-cov`, which needs the `llvm-tools-preview` component; it requires the `SONAR_TOKEN` secret (repo://.github/workflows/pr-gate.yml#L88-L127).

## Dependency audit — `audit.yml`

The only workflow on a schedule: Mondays at 03:00 UTC, plus manual dispatch. It runs `cargo-audit` for advisories and `cargo-deny` three times in parallel for `bans`, `licenses`, and `sources`, governed by `deny.toml` in the repository root (repo://.github/workflows/audit.yml#L4-L41, repo://deny.toml#L1).

The same file carries one job that is *pull-request-only* and therefore never runs on the schedule: `dependency-review` uses `actions/dependency-review-action` with `pull-requests: write` to comment on changed dependencies in an open PR (repo://.github/workflows/audit.yml#L42-L54). Because of that `if`, `workflow_dispatch` on this workflow skips the review job entirely.

## Release — `release.yml`

Triggered by a `v*` tag push, or manually by the repository owner supplying a tag (repo://.github/workflows/release.yml#L3-L12).

The `build` job carries an authorization condition that must hold for *anything* downstream to run: either the event is a tag push, or it is a manual dispatch whose actor is the repository owner. Every later job `needs: build`, so a skipped build gates the whole pipeline rather than publishing partial artifacts (repo://.github/workflows/release.yml#L15-L26).

On a tag push, the build first verifies that `v{Cargo.toml version}` equals the pushed tag and fails the job otherwise, which is what keeps release artifacts and crate version from drifting apart (repo://.github/workflows/release.yml#L52-L63). It then cross-compiles four targets — `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin` (on `macos-15-intel`), `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc` — packaging a `.tar.gz` per target and a `.zip` on Windows, then uploading each as a build artifact (repo://.github/workflows/release.yml#L28-L85).

The `checksum` job merges the downloaded artifacts and emits `SHA256SUMS.txt`; the `release` job, gated on `[build, checksum]`, creates the GitHub Release with `softprops/action-gh-release` using `GITHUB_TOKEN` and `contents: write` (repo://.github/workflows/release.yml#L87-L158).

## Documentation deployment — `openwiki-page.yml`

Renders the committed `openwiki/` tree into a static graph and reader using the pinned `openwiki@0.7.0` CLI on Node 22, then publishes it to GitHub Pages (repo://.github/workflows/openwiki-page.yml#L32-L64). There is no model call and no provider key here: the repository's wiki is the source of truth, and this workflow only renders it.

Two design points are deliberate. The export is asserted non-empty (`site/index.html`, `site/graph.json`, `site/client.js`) because `upload-pages-artifact` would otherwise publish an empty site and the failure would only surface as a 404 after deployment. And the `deploy` job is gated on `github.event_name != 'pull_request'`: pull requests build and upload the artifact but never publish, so unreviewed content cannot deploy itself while a broken export still fails on the change that caused it (repo://.github/workflows/openwiki-page.yml#L55-L84).

Related pages: [Test Suite Layout and Local Verification](../development/verification.md), [Running the Parser: Inputs, Options, and Runtime Cost](running-the-parser.md), [Quickstart](../quickstart.md).

Key resources: `repo://.github/workflows/pr-gate.yml`, `repo://.github/workflows/audit.yml`, `repo://.github/workflows/release.yml`, `repo://.github/workflows/openwiki-page.yml`, `repo://deny.toml`.
