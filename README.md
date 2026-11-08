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

## Träningsrutin (råden från chatten)

1. **Dragkontroll före varje drag:** Vad hotar motståndarens senaste drag? Hamnar pjäsen jag flyttar på en ruta där den kan tas gratis?
2. **Taktikpussel** 10–15 minuter om dagen, mest utdelning per minut.
3. **Längre partier**, 10–15 minuter, så det finns tid att tänka.

## Spec-utkast för appen (v1)

Minsta sak som fungerar. Inget mer än detta tills det används.

**Spela**
- Bräde på skärm och textinmatning av drag i notation. Båda går till samma brädmodell.
- Dragvalidering och ställning via ett befintligt schackbibliotek i Rust (t.ex. `shakmaty`), aldrig via Claude.
- Claudes drag väljs via en motor på begränsad styrka (Stockfish via UCI med satt Elo/skill level) så nivån följer min. Claude kommenterar, motorn flyttar.
- "Beskriv ställningen" i ord, för blindspel.

**Lära**
- Efter partiet: Claude pekar ut de 2–3 viktigaste felen (hängande pjäser, missade hot, missade taktiker) och sparar dem som lärdomar.
- Kunskapsbas: en enkel fil eller liten databas med partier (PGN), feltyper per parti och lärdomar.

**Följa upp**
- Grov Elo-uppskattning baserat på resultat mot motorn på känd styrka.
- Enkel graf: uppskattad Elo och antal enkla fel per parti över tid.

**Öppna frågor**
- Gränssnitt: CLI först, eller webb via wasm direkt?
- Var ska kunskapsbasen ligga: i repot, lokalt eller i Claudes minne?
- Röststyrning för drag när jag inte kan titta på skärmen?

## Utvecklingsmiljö

Kraven är hårda och körs både lokalt och i CI. Se [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) för hela listan.

```sh
make setup   # git-hooks och verktyg, en gång
make check   # allt som CI kör: fmt, clippy, typos, doc, test, 100 % täckning, arch, deny, machete, mutanter
```

- Toolchain pinnad i `rust-toolchain.toml`, varningar är fel.
- Clippy: all, pedantic, nursery, cargo samt utvalda restriction-lints (inga paniker, inget `unsafe`, inga `as`).
- 100 % täckning av rader, funktioner och regioner (cargo-llvm-cov).
- Inga överlevande mutanter (cargo-mutants).
- Conventional commits (committed), stavning (typos), beroenden (cargo-deny, cargo-machete), hemligheter (gitleaks).

## Status

Domänlagret finns med Elo-uppskattning och svensk notation, plus hela pipelinen. Nästa steg är att svara på de öppna frågorna och bygga spelloopen.
