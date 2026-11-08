# chess-trainer

En personlig schacktränare: spela mot Claude, bygg upp en kunskapsbas om mina styrkor och svagheter, och se hur jag blir bättre över tid. Skriven i Rust.

## Bakgrund

Idén kom ur en chatt med Claude (se [docs/ursprunglig-chatt.md](docs/ursprunglig-chatt.md)). Kort sammanfattat:

- Jag är nybörjare, gör enkla fel, men klarar normalt chess.coms bot på 1000 Elo.
- Jag vill spela mot Claude, inte mot chess.com.
- Claude ska minnas mina förmågor mellan partierna och anpassa sig, så att vi kan följa en ungefärlig Elo över tid.
- Jag kan inte alltid se ett bräde (t.ex. på cykeln), så drag i vanlig notation och ställningsbeskrivning i ord ska fungera.

## Beslut och avgränsningar från chatten

- **Claude är ingen schackmotor.** Brädet och dragvalidering sköts av kod, inte av Claude i huvudet. Claude står för pedagogik, nivåanpassning och lärdomar.
- **Ingen riktig Elo.** Vi för logg över partier och feltyper och ger en grov uppskattning som utvecklas över tid.
- **Notation.** Drag anges som "e4", "Sf3" eller "springare f3". Jag spelar vit i första partiet.
- **Efter varje parti** sparas två till tre lärdomar i kunskapsbasen.
- **Claude spelar på ungefär min nivå**, inte på max.
- **Så mycket som möjligt i Rust.**
- **Webbapp via wasm, kunskapsbasen i webbläsaren.**

## Träningsrutin (råden från chatten)

1. **Dragkontroll före varje drag:** Vad hotar motståndarens senaste drag? Hamnar pjäsen jag flyttar på en ruta där den kan tas gratis?
2. **Taktikpussel** 10–15 minuter om dagen, mest utdelning per minut.
3. **Längre partier**, 10–15 minuter, så det finns tid att tänka.

## Vad appen gör (v1)

En webbapp i Rust, kompilerad till wasm. Allt körs i webbläsaren och kunskapsbasen sparas där, i `localStorage`.

**Spela**
- Klicka på brädet eller skriv drag: "e4", "Sf3" eller "springare f3". Dragen visas med svenska bokstäver.
- Reglerna kommer från schackbiblioteket `cozy-chess` (MIT), aldrig från Claude. `shakmaty` valdes bort eftersom det är GPL.
- En egen motor i Rust spelar på din nivå. Den söker 1–3 halvdrag och väljer slumpvis bland drag som är nästan lika bra.
- "Beskriv ställningen" läser upp pjäserna i ord, för blindspel.

**Lära**
- Varje drag du gör bedöms. Tappar du mer än en och en halv bonde får du veta vad som var bättre och vilken sorts fel det var: hängande pjäs, missat slag eller missat hot.
- Felen sparas per parti. Lärdomarna visar de två till tre vanligaste felen i de senaste tio partierna, formulerade som dragkontrollen från chatten.

**Följa upp**
- Grov Elo-uppskattning efter varje parti, mot motorns nominella nivå. Startar på 1000.
- Motorns nivå följer din uppskattade rating.
- En kurva visar ratingen över tid.

**Inte med än**
- **Röststyrning.** Läggs till om du vill spela utan att titta på skärmen.
- **Kommentarer från Claude.** Felbedömningen är regelbaserad i dag. Claude kan kopplas in senare för förklaringar i fritext.
- **Kalibrering.** Nivåernas Elo-värden är gissningar. Spela mot chess.com-botar på känd nivå och justera tabellen i `crates/app/src/level.rs`.

## Spela lokalt

```sh
make serve   # öppna http://127.0.0.1:8080
```

## Utvecklingsmiljö

Kraven är hårda och körs både lokalt och i CI. Se [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) för hela listan.

```sh
make setup   # git-hooks och verktyg, en gång
make check   # allt som CI kör: fmt, clippy, typos, doc, test, 100 % täckning, arch, deny, machete, mutanter, wasm
```

- Toolchain pinnad i `rust-toolchain.toml`, varningar är fel.
- Clippy: all, pedantic, nursery, cargo samt utvalda restriction-lints (inga paniker, inget `unsafe`, inga `as`).
- 100 % täckning av rader, funktioner och regioner (cargo-llvm-cov).
- Inga överlevande mutanter (cargo-mutants).
- Conventional commits (committed), stavning (typos), beroenden (cargo-deny, cargo-machete), hemligheter (gitleaks).

## Status

v1 är spelbar: parti mot motorn, felbedömning, lärdomar och ratingkurva. Nästa steg är att kalibrera nivåerna och bestämma om röststyrning och Claude-kommentarer behövs.
