# Arkitektur

Allt som kan skrivas i Rust skrivs i Rust. Koden är en Cargo-arbetsyta med ett crate per lager.

## Lager

```
crates/domain   ren domänlogik utan beroenden: Elo, svensk notation
crates/engine   motor ovanpå cozy-chess: SAN, sökning med justerbar styrka, felbedömning
crates/app      användningsfall: parti, nivåanpassning, kunskapsbas, HTML-vyer
crates/web      tunt DOM-lim kompilerat till wasm, kunskapsbasen i localStorage
```

Beroenden pekar bara inåt: web → app → engine och domain. Cargo gör det omöjligt att bryta
utan att ändra en `Cargo.toml`, och `scripts/arch.sh` bevakar att domänet förblir helt utan
beroenden.

Hela appen körs i webbläsaren, utan server. Kunskapsbasen sparas som JSON i `localStorage`. Går den inte att läsa flyttas
den till en egen nyckel i stället för att skrivas över.

Webbcratet är undantaget från täckning och mutationstest, eftersom det bara går att köra i
en webbläsare. Därför innehåller det ingen logik; allt som går att testa ligger i `app`.
I stället bygger CI webbappen med trunk.

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
| Webbappen bygger till wasm               | trunk                                        |
| Pinnad toolchain och actions             | `rust-toolchain.toml`, SHA-pinnade actions, Dependabot |

## Lokalt

```sh
make setup   # git-hooks och verktyg, en gång
make check   # exakt det CI kör
```

Pre-commit kör formatering, lint och stavning. Pre-push kör `make check`.

## Avsiktliga förenklingar

Markerade med `// ponytail:` i koden, med tak och uppgraderingsväg i kommentaren.
