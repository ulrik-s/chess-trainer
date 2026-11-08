# Arkitektur

Allt som kan skrivas i Rust skrivs i Rust. Koden är en Cargo-arbetsyta med ett crate per lager.

## Lager

```
crates/domain     ren domänlogik, inga beroenden, ingen I/O
crates/app        (kommer) användningsfall: spela parti, analysera, uppdatera kunskapsbas
crates/adapters   (kommer) motor (Stockfish via UCI), lagring, UI (CLI, senare webb via wasm)
```

Beroenden pekar bara inåt: adapters → app → domain. Cargo gör detta omöjligt att bryta
utan att ändra en `Cargo.toml`, och `scripts/arch.sh` bevakar att domänet förblir helt
utan beroenden.

## Regler som bevakas automatiskt

| Regel                                    | Verktyg                                      |
| ---------------------------------------- | -------------------------------------------- |
| Formatering                              | rustfmt, taplo (TOML)                        |
| Lint: clippy all + pedantic + nursery + cargo, utvalda restriction | clippy via `[workspace.lints]` |
| Varningar är fel                         | `.cargo/config.toml` (`-D warnings`)         |
| Ingen `unsafe`, inga paniker, inga `as`  | `forbid(unsafe_code)`, clippy restriction    |
| Dokumentation på allt, även privat       | `missing_docs`, `missing_docs_in_private_items`, rustdoc utan varningar |
| 100 % rader, funktioner och regioner     | cargo-llvm-cov                               |
| Inga överlevande mutanter                | cargo-mutants                                |
| Sårbarheter, licenser, dubbletter, källor | cargo-deny                                  |
| Oanvända beroenden                       | cargo-machete                                |
| Domänet utan beroenden                   | `scripts/arch.sh`                            |
| Stavning                                 | typos                                        |
| Conventional commits                     | committed (commit-msg-hook och CI)           |
| Hemligheter i historiken                 | gitleaks                                     |
| Workflow-syntax                          | actionlint                                   |
| Pinnad toolchain och actions             | `rust-toolchain.toml`, SHA-pinnade actions, Dependabot |

## Lokalt

```sh
make setup   # git-hooks och verktyg, en gång
make check   # exakt det CI kör
```

Pre-commit kör formatering, lint och stavning. Pre-push kör `make check`.

## Avsiktliga förenklingar

Markerade med `// ponytail:` i koden, med tak och uppgraderingsväg i kommentaren.
