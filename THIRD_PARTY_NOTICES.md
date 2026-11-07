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
