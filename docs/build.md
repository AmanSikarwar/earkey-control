# Build and reproducibility conventions

Stages 2 and 3 have Rust libraries and locked host checks. The Android executable, installable package and CI workflow are later gates.

## Stage 2 host checks

The workspace pins Rust 1.99.1 (rustc `58cf8f9c3`, Cargo `5f94df478`) with rustfmt and Clippy. Direct dependencies use exact versions and the [Cargo lockfile](../Cargo.lock) records registry checksums and transitive versions. Host checks used arm64 macOS 27.2 with Darwin 27.2.0. The [dependency inventory](../THIRD_PARTY_NOTICES.md) records package roles and license files.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Use `--offline` for Clippy and tests after fetching the locked dependencies. These checks exercise the Rust model and the host filesystem. They establish no Android loading, SELinux access or input behavior.

## Stage 3 host fuzzing and sanitizer

The host evidence uses Rust nightly-2026-11-06 (`249c1e4e3`) for instrumentation and cargo-fuzz 0.13.2 with libfuzzer-sys 0.4.13. The production toolchain pin stays on Rust 1.99.1. The fuzz workspace has a separate [lockfile](../fuzz/Cargo.lock) and its dependencies are host-only. The recorded host was arm64 macOS 27.2 with Apple Clang 21.0.0 (`clang-2100.4.1.1`). The sanitizer run instruments the project and dependency crates. [Rust sanitizer support](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html), [cargo-fuzz 0.13.2](https://github.com/rust-fuzz/cargo-fuzz/releases/tag/0.13.2)

```sh
rustup toolchain install nightly-2026-11-06 --profile minimal
cargo install cargo-fuzz --version '=0.13.2' --locked
cargo fetch --locked --manifest-path fuzz/Cargo.toml
RUSTFLAGS='-Zsanitizer=address' cargo +nightly-2026-11-06 test --workspace --locked --target aarch64-apple-darwin
cargo clippy --manifest-path fuzz/Cargo.toml --bins --locked -- -D warnings
```

Seed the campaigns with the synthetic policy and protocol fixtures. The eight-byte event encoding below belongs to the harness. Native Linux record layout is an adapter concern.

```sh
earkey_corpus=$(mktemp -d)
mkdir -p "$earkey_corpus/policy" "$earkey_corpus/events"
cp tests/fixtures/identity/reference-policy.json "$earkey_corpus/policy/"
python3 - "$earkey_corpus/events" <<'PYTHON'
import json, pathlib, struct, sys

out = pathlib.Path(sys.argv[1])
fixtures = json.loads(pathlib.Path("tests/fixtures/protocol/synthetic-event-summaries.v1.json").read_text())
for case in fixtures["cases"]:
    records = b"".join(
        struct.pack("<HHI", *(int(value, 16) for value in row[1:]))
        for row in case["events"]
    )
    for mask in (0, 1, 7):
        (out / f"{case['id']}-{mask}").write_bytes(bytes([mask]) + records)
PYTHON
CARGO_NET_OFFLINE=true cargo +nightly-2026-11-06 fuzz run policy "$earkey_corpus/policy" -- -runs=100000 -max_len=16385 -seed=13494 -timeout=3 -print_final_stats=1
CARGO_NET_OFFLINE=true cargo +nightly-2026-11-06 fuzz run events "$earkey_corpus/events" -- -runs=100000 -max_len=4097 -seed=13494 -timeout=3 -print_final_stats=1
```

Check that the fuzz lockfile stays unchanged. Keep generated corpora and crash artifacts outside the tracked tree. The event harness checks output order, report boundaries and allowed press/repeat/release state.

## Pins and native target

Before each corresponding build, pin the toolchain and inputs. Android and package builds still need the NDK revision/download digest, WebUI compiler and bridge dependencies and lockfile, build host/container identity, packaging tools and any upstream installer revision. Third-party Actions require reviewed full commit SHAs. Record sources, versions, license notices, patches and research date; no mutable latest dependency, hidden download or boot-time executable fetch.

Start with the **measured** reference ABI and API 31 native minimum. Use Cargo and the Android NDK linker for Android targets; a desktop Linux executable is not an Android binary. Inspect ELF machine, interpreter, dynamic dependencies and imported API availability; then test actual loading. Add ABIs only with runtime demand and a test plan. [Rust Android target guidance](https://doc.rust-lang.org/rustc/platform-support/android.html) and [NDK API availability](https://developer.android.com/ndk/guides/using-newer-apis) govern the implementation.

Configure and inspect native executable and bundled-library ELF alignment for 16 KiB systems and avoid fixed-page assumptions. NDK defaults alone are insufficient proof for the Rust link and dependencies. Test loading on 4 KiB and 16 KiB environments before claiming them. APK ZIP alignment concerns an optional APK, not the root-module archive. See [Android page-size guidance](https://developer.android.com/guide/practices/page-sizes).

## Artifact construction

Build from one clean exact reviewed commit. Stage only reviewed runtime assets: root `module.prop`, supported lifecycle/customization scripts, `skip_mount`, ABI-labelled binaries, bundled `webroot` resources, safe defaults/schema, project license and actual third-party notices. Include a narrowly justified `sepolicy.rule` only if real service denials prove necessity. Use documented manager installer conventions and pin any required upstream wrapper; no conflicting `install.sh` or invented privileged update-binary behavior.

Normalize entry order, timestamps, modes and LF endings. Inspect ZIP root level, executable modes, safe inactive defaults, version/versionCode consistency, inventory and absence of private data, unknown binaries, repository wrapper directories and online payload dependencies. Archive/source download is not an installable module. Build twice in clean environments and compare final ZIP/native digests; investigate differences before claiming byte-for-byte reproducibility. Record commit, exact recipe, tool/dependency identities, asset inventory, notices, checksums and available provenance/SBOM.

## Planned CI

Introduce checks alongside their actual source: formatting/static analysis, ash syntax/ShellCheck, policy/schema/migrations, identity/event-state tests, parser fuzz regressions, applicable sanitizers, UI build/accessibility/offline validation, manager fixtures, per-distributed-ABI ELF and package checks. Use stable unique names and a final required status that fails for failed or unexpectedly skipped mandatory dependencies. Do not configure non-existent required checks or count a placeholder job as passing.

PR jobs have read-only contents permissions and no release secrets or personal phone access. Isolate release write/attestation privileges, validate inputs, pin third-party Actions to reviewed full SHAs, and never run untrusted PR code via privileged pull_request_target. Hardware runs use reviewed code under maintainer control. Host checks do not replace [physical validation](validation-plan.md).
