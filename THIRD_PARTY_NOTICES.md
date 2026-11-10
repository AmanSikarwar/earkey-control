# Dependency notice record

No third-party code, binary, installer wrapper, JavaScript package, font, or icon has been vendored or selected as a runtime dependency in Stage 0. Linking documentation does not select a package or satisfy redistribution obligations.

The maintainer supplied the specification as project requirements and authorized a public project repository. It is retained unchanged with its source digest recorded in the contract. The project's selected [license is MIT](LICENSE); upstream dependencies retain their own licenses when selected.

When selecting an actual dependency, record its name, exact version or immutable revision, source URL, digest, license expression, included license/notice text, modifications, build role, and redistribution requirements. Include transitive runtime dependencies and any copied manager installer wrapper. Generate release notices from reviewed lockfiles and the packaged asset inventory; do not infer them from research links.

## Stage 2 Rust inventory

The locked packages below were reviewed on 7 November 2026. The [lockfile](Cargo.lock) records each registry package SHA-256. Crates are used unmodified and fetched as build inputs. No dependency source or native binary is bundled in this change. Release packaging must include the applicable license and copyright texts from the exact archives for distributed components and check the final asset inventory.

| Package | Build role | License expression | License and notice files in archive |
| --- | --- | --- | --- |
| [itoa 1.0.18](https://crates.io/crates/itoa/1.0.18) | Runtime JSON integer formatting | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [libc 0.2.191](https://crates.io/crates/libc/0.2.191) | Runtime Unix filesystem bindings | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [memchr 2.8.3](https://crates.io/crates/memchr/2.8.3) | Runtime JSON byte scanning | `Unlicense OR MIT` | `COPYING`, `LICENSE-MIT`, `UNLICENSE` |
| [proc-macro2 1.0.107](https://crates.io/crates/proc-macro2/1.0.107) | Host derive build | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [quote 1.0.47](https://crates.io/crates/quote/1.0.47) | Host derive build | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [serde 1.0.229](https://crates.io/crates/serde/1.0.229) | Runtime typed serialization | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [serde_core 1.0.229](https://crates.io/crates/serde_core/1.0.229) | Runtime serialization traits | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [serde_derive 1.0.229](https://crates.io/crates/serde_derive/1.0.229) | Host derive build | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [serde_json 1.0.154](https://crates.io/crates/serde_json/1.0.154) | Runtime JSON codec | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [syn 3.0.9](https://crates.io/crates/syn/3.0.9) | Host derive build | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [unicode-ident 1.0.27](https://crates.io/crates/unicode-ident/1.0.27) | Host derive build | `(MIT OR Apache-2.0) AND Unicode-3.0` | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-UNICODE` |
| [zmij 1.0.23](https://crates.io/crates/zmij/1.0.23) | Runtime JSON number formatting | `MIT` | `LICENSE-MIT` |

## Stage 4 worker layout digest

The locked packages below were added on 10 November 2026 so the worker can hash the key layout Android selects. The maintainer chose the pinned `sha2` crate over a local SHA-256 implementation. `sha2` is used with default features disabled, so `alloc` and `oid` are off. Current `sha2` documentation was checked through Context7 (`/websites/rs_sha2`). The [lockfile](Cargo.lock) records each registry package SHA-256. The crates are used unmodified, and `cpufeatures` reuses the already-recorded `libc 0.2.191`. Distributed binaries must include these license texts.

| Package | Build role | License expression | License and notice files in archive |
| --- | --- | --- | --- |
| [sha2 0.11.1](https://crates.io/crates/sha2/0.11.1) | Runtime SHA-256 | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [digest 0.11.3](https://crates.io/crates/digest/0.11.3) | Runtime hash traits | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [block-buffer 0.12.1](https://crates.io/crates/block-buffer/0.12.1) | Runtime block buffering | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [crypto-common 0.2.2](https://crates.io/crates/crypto-common/0.2.2) | Runtime cryptographic traits | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [hybrid-array 0.4.15](https://crates.io/crates/hybrid-array/0.4.15) | Runtime fixed-size arrays | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [typenum 1.20.1](https://crates.io/crates/typenum/1.20.1) | Compile-time type-level numbers | `MIT OR Apache-2.0` | `LICENSE`, `LICENSE-APACHE`, `LICENSE-MIT` |
| [cpufeatures 0.3.1](https://crates.io/crates/cpufeatures/0.3.1) | Runtime CPU feature detection | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [cfg-if 1.0.5](https://crates.io/crates/cfg-if/1.0.5) | Compile-time configuration | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |

## Stage 3 host fuzz tooling

The campaign used [cargo-fuzz 0.13.2](https://crates.io/crates/cargo-fuzz/0.13.2), installed with its upstream lockfile. The separate [fuzz lockfile](fuzz/Cargo.lock) pins and records checksums for the harness crates. The packages below are additional host tooling and are outside the production workspace. No package has been modified or bundled.

| Package | License expression | License and notice files in archive |
| --- | --- | --- |
| [arbitrary 1.5.0](https://crates.io/crates/arbitrary/1.5.0) | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [cc 1.11.1](https://crates.io/crates/cc/1.11.1) | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [cfg-if 1.0.5](https://crates.io/crates/cfg-if/1.0.5) | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [find-msvc-tools 0.1.15](https://crates.io/crates/find-msvc-tools/0.1.15) | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [getrandom 0.4.3](https://crates.io/crates/getrandom/0.4.3) | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [jobserver 0.1.35](https://crates.io/crates/jobserver/0.1.35) | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [libfuzzer-sys 0.4.13](https://crates.io/crates/libfuzzer-sys/0.4.13) | `(MIT OR Apache-2.0) AND NCSA` | `LICENSE-APACHE`, `LICENSE-MIT` |
| [r-efi 6.0.0](https://crates.io/crates/r-efi/6.0.0) | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | Expression recorded in the package manifest |
| [shlex 2.0.1](https://crates.io/crates/shlex/2.0.1) | `MIT OR Apache-2.0` | `LICENSE-APACHE`, `LICENSE-MIT` |
