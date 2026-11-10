# Build and reproducibility conventions

Stages 2–4 have Rust libraries and locked host checks: the policy/identity store, the event engine and the supervisor state machine. Stage 4 also adds a standalone Android capability probe. The passthrough executable, installable package and CI workflow are later gates.

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

## Stage 4 host supervisor checks

The supervisor crate is a pure state machine; its tests use synthetic monotonic timestamps and signals. They exercise start, progress, pause, kill and restart deadlines, the three-failure latch, explicit recovery and the pause/boot-escape gate. No process is spawned, no descriptor is opened and no deadline is measured.

```sh
cargo test -p earkey-supervisor --locked
```

The workspace-wide commands above also run these tests. A passing host suite does not establish process identity, descriptor release, Android service behavior or physical pause/recovery.

The Linux process adapter tests drive real `/bin/sh` workers, so they compile only for Linux and Android. On macOS, run them in the official Rust 1.99.1 Alpine arm64 image `rust@sha256:0a555e8b51945db91fd7b8411b4272bb5b8bfa04261d31eaeb2ada564bc02632`. Use no network, dropped capabilities, a read-only root and the host registry mounted read-only:

```sh
docker run --rm --network none --cap-drop ALL --read-only --tmpfs /tmp:exec,size=1g \
  -e RUSTUP_TOOLCHAIN=1.99.1-aarch64-unknown-linux-musl -e CARGO_TARGET_DIR=/tmp/target -e CARGO_HOME=/tmp/cargo \
  -v "$HOME/.cargo/registry:/tmp/cargo/registry:ro" -v "$PWD:/src:ro" -w /src \
  rust@sha256:0a555e8b51945db91fd7b8411b4272bb5b8bfa04261d31eaeb2ada564bc02632 \
  cargo test --workspace --locked --offline
```

To run the drill end to end, add `--tmpfs /data:size=16m` to the command above. Then replace the cargo command with a shell that builds `earkey-drill`, creates `/data/adb` (mode `0755`) and `/data/adb/earkey_control` (mode `0700`), and runs `/tmp/target/debug/earkey-drill drill`. The container runs as root without capabilities, which satisfies the store's ownership checks.

The worker inventory's node tests create character devices, so they need `--cap-add MKNOD` and a `/tmp` tmpfs mounted with `exec,dev`. They fail rather than skip when `mknod` is denied.

The real uinput proxy test is ignored by default because it creates a virtual input device in the test kernel. The Docker Desktop LinuxKit kernel has `CONFIG_INPUT_UINPUT=y`. Run it there by adding `--cap-add MKNOD --device /dev/uinput` and passing `-- --include-ignored` to `cargo test -p earkey-worker --test proxy`. Never run it on the reference phone. The forwarding tests also open input event nodes, so they additionally need `--device-cgroup-rule 'c 13:* rmw'`.

`RUSTUP_TOOLCHAIN` selects the image's installed toolchain instead of syncing `rust-toolchain.toml` components. `cargo clippy -p earkey-supervisor --all-targets --target aarch64-linux-android -- -D warnings` type-checks the adapter and its tests for the Android target.

## Stage 4 Android capability probe

The prototype host pin uses the existing stable r28c SDK package `28.2.13676358` on arm64 macOS 27.2 and Rust 1.99.1 with the `aarch64-linux-android` target. The official SDK index lists `android-ndk-r28c-darwin.zip` at 952,495,160 bytes with SHA-1 `fc20a6bf15a30fb3428c9b60a7308793a362dc6d`. This identifies the upstream archive. The original download was absent locally, so the build checks the installed package revision and hashes the tool binaries used. Android now archives r28c. This pin applies to the initial probe and must be revisited before release qualification. [Official SDK index](https://dl.google.com/android/repository/repository2-3.xml), [NDK r28c release](https://github.com/android/ndk/releases/tag/r28c)

| Tool from the installed Darwin package | SHA-256 |
| --- | --- |
| `clang` 19.0.1, build 13624864, LLVM `97a699bf4812a18fb657c2779f5296a4ab2694d2` | `df85444b66234bf4cae267e22bde45ea8fef596d30ca2991b2091a27e6ea7718` |
| `ld.lld` | `295bc36e1b10be0137f09904b7cc928a11ab3d44a8ec8250e4087ebd71f091c2` |
| `llvm-readelf` | `37e565359be0c9f2868348dd314416a420d137ee84c891ec8474cf7d29cfd995` |

Build from a clean reviewed checkout. Set `ANDROID_NDK_HOME` to the installed pinned package. The macOS SDK uses the directory name `darwin-x86_64` for its universal tools.

```sh
rustup target add --toolchain 1.99.1 aarch64-linux-android
earkey_prebuilt="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64"
test "$(sed -n 's/^Pkg.Revision = //p' "$ANDROID_NDK_HOME/source.properties")" = 28.2.13676358
shasum -a 256 "$earkey_prebuilt/bin/clang" "$earkey_prebuilt/bin/ld.lld" "$earkey_prebuilt/bin/llvm-readelf"
export CARGO_TARGET_DIR="$(mktemp -d)"
earkey_registry="${CARGO_HOME:-$HOME/.cargo}"
earkey_sysroot="$(rustc --print sysroot)"
earkey_flags=$(printf '%s\037%s\037%s\037%s\037%s' \
  '-Clink-arg=-Wl,-z,max-page-size=16384' '-Cstrip=symbols' \
  "--remap-path-prefix=$(pwd -P)=/earkey-control" "--remap-path-prefix=$earkey_registry=/cargo" \
  "--remap-path-prefix=$earkey_sysroot=/rust-sysroot")
CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$earkey_prebuilt/bin/aarch64-linux-android31-clang" \
  CARGO_ENCODED_RUSTFLAGS="$earkey_flags" \
  cargo build --release --locked --target aarch64-linux-android -p earkey-probe
earkey_probe="$CARGO_TARGET_DIR/aarch64-linux-android/release/earkey-probe"
"$earkey_prebuilt/bin/llvm-readelf" -h -l -d -n --dyn-syms --wide "$earkey_probe"
shasum -a 256 "$earkey_probe"
```

Compare the recorded tool hashes before building. Expect an AArch64 Android PIE with API 31 identification, `/system/bin/linker64`, and load segments aligned to at least `0x4000`. Compare imported symbols against the NDK API 31 libraries and scan the binary for host paths. The sysroot remap also covers source paths emitted when Rust specializes standard-library code. The probe reports the Rust binding's `input_event` size and field offset, checked against the r28c UAPI header for arm64. Static inspection establishes the ELF properties. Actual loading and service access require an authorized device run. [Other build systems](https://developer.android.com/ndk/guides/other_build_systems), [Android page sizes](https://developer.android.com/guide/practices/page-sizes)

## Stage 4 Android supervision drill

`earkey-drill` uses the same pinned r28c tools, recorded tool hashes, flags and checks as the probe. Replace `-p earkey-probe` with `-p earkey-drill` and inspect `$CARGO_TARGET_DIR/aarch64-linux-android/release/earkey-drill`.

On 10 November 2026 two clean builds from `6db695d` produced identical 576,040-byte ELFs, SHA-256 `156a0391538fc0ceec18fc2f68144136215d885d9a2696feb11f5a98d9bbc475`. The NDK revision and the three tool hashes matched the pins. `llvm-readelf` showed an AArch64 PIE using `/system/bin/linker64`, with the Android note at API 31 and NDK r28c, all `LOAD` segments aligned to `0x4000`, and `BIND_NOW`. It needs only `libc.so` and `libdl.so`, and all 85 imported symbols are defined by the NDK API 31 `libc`, `libdl` and `libm` stubs. A string scan found no host paths. The ELF is kept outside Git at `target/stage4/6db695d/earkey-drill-arm64-api31`. Static inspection does not establish loading or behavior on the phone.

## Stage 4 Android worker check

`earkey-worker` uses the same pinned r28c tools, recorded tool hashes, flags and checks as the probe, built with `-p earkey-worker`. On 10 November 2026 two clean builds from `a0f45ee` produced identical 572,608-byte ELFs, SHA-256 `7ee31586bf546e696c1e180d5766eaa541ab1628b3f35bd0d091a041c329a4c9`. The NDK revision and the three tool hashes matched the pins. `llvm-readelf` showed an AArch64 PIE using `/system/bin/linker64`, with the Android note at API 31, all four `LOAD` segments aligned to `0x4000`, and `BIND_NOW`. It needs only `libc.so` and `libdl.so`, and all 55 imported symbols are defined by the NDK API 31 stubs. A string scan found no host paths. The ELF is kept outside Git at `target/stage4/a0f45ee/earkey-worker-arm64-api31`.

For v17, two clean builds from `4938f7f` produced identical 774,216-byte ELFs, SHA-256 `5953973300dacd3be36f72fb4511ce13e8ebb70a53e868e760ef3d3107fc2cff`, with the same pins and checks (API 31, 16 KiB-aligned segments, only `libc`/`libdl`, all 90 imports in API 31, no host paths). It is kept at `target/stage4/4938f7f/earkey-worker-arm64-api31`.

For v16, two clean builds from `7af4c5d` produced identical 763,952-byte ELFs, SHA-256 `9cd7ff90e5b9cdab1f7600c2a82226f395f5e66aec027b312eea83140ebc173e`, with the same pins and checks (API 31, 16 KiB-aligned segments, only `libc`/`libdl`, all 90 imports in API 31, no host paths). It is kept at `target/stage4/7af4c5d/earkey-worker-arm64-api31`.

For v15, two clean builds from `f3cc75c` produced identical 741,728-byte ELFs, SHA-256 `e6a32fca5671f4d0831bb9615e5d7c7cf4adf2a0824a012fcf8767a4e93ed091`, with the same pins and checks: API 31, 16 KiB-aligned segments, only `libc`/`libdl`, all 90 imports in API 31, and no host paths. The ELF is kept at `target/stage4/f3cc75c/earkey-worker-arm64-api31`.

For v13, two clean builds from `8035136` produced identical 672,792-byte ELFs, SHA-256 `57178e5f0431f85ac0214505381a460b3cdb0a6953191f51bfea9aa085b4e895`. The same pins and checks passed: AArch64 PIE using `linker64`, API 31, four `LOAD` segments aligned to `0x4000`, only `libc.so` and `libdl.so`, all 82 imports in the API 31 stubs, and no host paths. The ELF is kept at `target/stage4/8035136/earkey-worker-arm64-api31`.

## Stage 6 WebUI

`webui/` is TypeScript compiled by TypeScript 6.0.3, pinned exactly in `package.json` and by integrity in `package-lock.json`. TypeScript is build-only, and the page has no runtime dependencies, so no WebUI code is bundled from third parties. Node 26.13.0 and npm 11.20.0 built it on 11 November 2026.

```sh
cd webui
npm ci --ignore-scripts
npm test   # compiles to dist/ and runs node:test on the status logic
```

`tools/package-module.py` runs `npm run build` and copies `index.html`, `style.css`, `main.js` and `status.js` to `webroot/`. It fails if `node_modules` is missing or the output differs.

## Pins and native target

Before each corresponding build, pin the toolchain and inputs. Release builds still need a qualified NDK revision and archive digest, WebUI compiler and bridge dependencies and lockfile, build host/container identity, packaging tools and any upstream installer revision. Third-party Actions require reviewed full commit SHAs. Record sources, versions, license notices, patches and research date; no mutable latest dependency, hidden download or boot-time executable fetch.

Start with the **measured** reference ABI and API 31 native minimum. Use Cargo and the Android NDK linker for Android targets; a desktop Linux executable is not an Android binary. Inspect ELF machine, interpreter, dynamic dependencies and imported API availability; then test actual loading. Add ABIs only with runtime demand and a test plan. [Rust Android target guidance](https://doc.rust-lang.org/rustc/platform-support/android.html) and [NDK API availability](https://developer.android.com/ndk/guides/using-newer-apis) govern the implementation.

Configure and inspect native executable and bundled-library ELF alignment for 16 KiB systems and avoid fixed-page assumptions. NDK defaults alone are insufficient proof for the Rust link and dependencies. Test loading on 4 KiB and 16 KiB environments before claiming them. APK ZIP alignment concerns an optional APK, not the root-module archive. See [Android page-size guidance](https://developer.android.com/guide/practices/page-sizes).

## Artifact construction

Build from one clean exact reviewed commit. Stage only reviewed runtime assets: root `module.prop`, supported lifecycle/customization scripts, `skip_mount`, ABI-labelled binaries, bundled `webroot` resources, safe defaults/schema, project license and actual third-party notices. Include a narrowly justified `sepolicy.rule` only if real service denials prove necessity. Use documented manager installer conventions and pin any required upstream wrapper; no conflicting `install.sh` or invented privileged update-binary behavior.

Normalize entry order, timestamps, modes and LF endings. Inspect ZIP root level, executable modes, safe inactive defaults, version/versionCode consistency, inventory and absence of private data, unknown binaries, repository wrapper directories and online payload dependencies. Archive/source download is not an installable module. Build twice in clean environments and compare final ZIP/native digests; investigate differences before claiming byte-for-byte reproducibility. Record commit, exact recipe, tool/dependency identities, asset inventory, notices, checksums and available provenance/SBOM.

## Planned CI

Introduce checks alongside their actual source: formatting/static analysis, ash syntax/ShellCheck, policy/schema/migrations, identity/event-state tests, parser fuzz regressions, applicable sanitizers, UI build/accessibility/offline validation, manager fixtures, per-distributed-ABI ELF and package checks. Use stable unique names and a final required status that fails for failed or unexpectedly skipped mandatory dependencies. Do not configure non-existent required checks or count a placeholder job as passing.

PR jobs have read-only contents permissions and no release secrets or personal phone access. Isolate release write/attestation privileges, validate inputs, pin third-party Actions to reviewed full SHAs, and never run untrusted PR code via privileged pull_request_target. Hardware runs use reviewed code under maintainer control. Host checks do not replace [physical validation](validation-plan.md).
