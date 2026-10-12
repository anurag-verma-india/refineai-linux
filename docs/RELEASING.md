# Releasing RefineAI

How to cut a release and publish prebuilt binaries on GitHub, so users can
install with the one-line script in the [README](../README.md#install).

The release flow is: **build a stripped release binary → tag → create a GitHub
Release → upload the binary as an asset**. The user installer
([`scripts/install.sh`](../scripts/install.sh)) downloads that asset, so the
asset names below must match what the script expects.

## TL;DR — one command

[`release.sh`](../release.sh) does the whole flow — bump, build, test, tag, push,
and publish — taking the details as arguments. It needs `cargo`, `git`, and the
authenticated [GitHub CLI](https://cli.github.com/) (`gh auth login`).

```bash
# Simplest: version only (title + notes auto-generated)
./release.sh 0.1.0

# With a title and notes, plus an aarch64 build, no prompt
./release.sh 0.1.0 \
  --title "RefineAI v0.1.0" \
  --notes "First public release." \
  --aarch64 --yes
```

`./release.sh --help` lists every flag (`--notes-file`, `--draft`,
`--prerelease`, `--skip-tests`, `--no-push`, `-y/--yes`). The rest of this doc
is the manual flow it automates — read it to understand each step, to publish
without `gh`, or to debug a release.

## Asset naming

The installer downloads `refineai-<arch>` from the release, where `<arch>` is
the output of `uname -m`:

| CPU | `uname -m` | Asset name |
| --- | --- | --- |
| 64-bit Intel/AMD | `x86_64` | `refineai-x86_64` |
| 64-bit ARM | `aarch64` | `refineai-aarch64` |

Ship at least `refineai-x86_64`. Add `refineai-aarch64` if you can build it
(see [Cross-compiling](#cross-compiling-aarch64)).

## 1. Bump the version

Update `version` in [`Cargo.toml`](../Cargo.toml), then refresh the lockfile and
commit:

```bash
cargo build                      # updates Cargo.lock
git commit -am "release: v0.1.0"
```

Use [semver](https://semver.org/): `MAJOR.MINOR.PATCH`. The git tag is this
version prefixed with `v` (e.g. `v0.1.0`).

## 2. Build the release binary

```bash
cargo build --release
```

The result is `target/release/refineai` — already stripped and size-optimized
(`opt-level = "z"`, `lto`, `strip`, `panic = "abort"` in `Cargo.toml`), ~7.7 MB.

Rename it to the asset name for your arch:

```bash
cp target/release/refineai refineai-x86_64
```

### Cross-compiling aarch64

With [`cross`](https://github.com/cross-rs/cross) (needs Docker/Podman):

```bash
cargo install cross
cross build --release --target aarch64-unknown-linux-gnu
cp target/aarch64-unknown-linux-gnu/release/refineai refineai-aarch64
```

## 3. Tag and push

```bash
git tag -a v0.1.0 -m "RefineAI v0.1.0"
git push origin main --tags
```

## 4. Create the GitHub Release

### With the `gh` CLI (recommended)

One command tags (if needed), creates the release, and uploads the assets:

```bash
gh release create v0.1.0 \
  refineai-x86_64 \
  refineai-aarch64 \
  --title "RefineAI v0.1.0" \
  --notes "First public release. See README for install."
```

Generate notes from merged PRs/commits instead with `--generate-notes`.

### In the web UI

1. **Releases → Draft a new release**.
2. Choose the tag `v0.1.0` (create it on publish if you didn't push one).
3. Title + notes.
4. Drag `refineai-x86_64` (and `refineai-aarch64`) into **Attach binaries**.
5. **Publish release**.

## 5. Verify the one-line install

`releases/latest/download/...` always points at the newest published release,
so test the exact command users will run:

```bash
curl -fsSL https://raw.githubusercontent.com/anurag-verma-india/refineai-linux/main/scripts/install.sh | bash
refineai --help
```

Pin-to-version and uninstall also work:

```bash
curl -fsSL .../scripts/install.sh | bash -s -- --version v0.1.0
curl -fsSL .../scripts/install.sh | bash -s -- --uninstall
```

## Checklist

- [ ] `cargo fmt` and `cargo clippy` are clean
- [ ] `cargo test` passes
- [ ] `version` bumped in `Cargo.toml`, `Cargo.lock` refreshed
- [ ] release binary built and renamed to `refineai-<arch>`
- [ ] tag `vX.Y.Z` pushed
- [ ] GitHub Release created with the binary asset(s) attached
- [ ] one-line install verified on a clean machine

## Packaging for APT

Shipping to Ubuntu/Debian users through `apt` is a separate track — see
[`docs/PACKAGING-APT.md`](./PACKAGING-APT.md).
