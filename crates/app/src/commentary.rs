//! Motorns kommentar till sitt eget drag: vad draget gör, hur den bedömer ställningen
//! och vilka andra drag den övervägde. Skrivs i första person, som om motståndaren tänkte högt.

use chess_trainer_domain::notation::to_swedish;
use chess_trainer_engine::explain::{Idea, ideas};
use chess_trainer_engine::san::san;
use chess_trainer_engine::search::{Choice, MATE};
use chess_trainer_engine::{Board, Piece};

/// Så många andra drag nämns.
const ALTERNATIVES: usize = 2;

/// Så många centibönder sämre än det bästa draget måste valet vara för att motorn ska erkänna det.
const ADMIT_WORSE: i32 = 30;

/// Pjäsens namn i obestämd form.
const fn name(piece: Piece) -> &'static str {
    match piece {
        Piece::Pawn => "bonde",
        Piece::Knight => "springare",
        Piece::Bishop => "löpare",
        Piece::Rook => "torn",
        Piece::Queen => "dam",
        Piece::King => "kung",
    }
}

/// "din springare" eller "ditt torn".
fn yours(piece: Piece) -> String {
    let pronoun = if piece == Piece::Rook { "ditt" } else { "din" };
    format!("{pronoun} {}", name(piece))
}

/// "min springare" eller "mitt torn".
fn mine(piece: Piece) -> String {
    let pronoun = if piece == Piece::Rook { "mitt" } else { "min" };
    format!("{pronoun} {}", name(piece))
}

/// En mening för en idé.
fn sentence(idea: Idea) -> String {
    match idea {
        Idea::Mate => "Det är schack matt!".to_owned(),
        Idea::Check => "Det ger schack.".to_owned(),
        Idea::Castles => "Jag rockerar för att få kungen i säkerhet.".to_owned(),
        Idea::Captures(piece) => format!("Jag slår {}.", yours(piece)),
        Idea::Escapes(piece) => {
            let threatened = if piece == Piece::Rook {
                "hotat"
            } else {
                "hotad"
            };
            format!(
                "{} var {threatened}, så jag flyttar undan den.",
                capitalise(&mine(piece))
            )
        }
        Idea::Threatens(piece, square) => format!("Nu hotar jag {} på {square}.", yours(piece)),
        Idea::Develops(piece) => format!("Jag utvecklar {} mot mitten.", mine(piece)),
        Idea::TakesCenter => "Bonden tar plats i centrum.".to_owned(),
    }
}

/// Första bokstaven versal.
fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// Bedömningen i ord. `score` är i centibönder ur motorns perspektiv.
fn assessment(score: i32) -> String {
    if score > MATE / 2 {
        return "Jag ser en väg till matt.".to_owned();
    }
    if score < -(MATE / 2) {
        return "Jag ser att du kan göra matt. Hittar du det?".to_owned();
    }
    let pawns = format!("{:+.1}", f64::from(score) / 100.0).replace('.', ",");
    match score {
        s if s >= 150 => format!("Jag tycker att jag står klart bättre ({pawns})."),
        s if s >= 50 => format!("Jag tycker att jag står lite bättre ({pawns})."),
        s if s > -50 => format!("Ställningen känns jämn ({pawns})."),
        s if s > -150 => format!("Jag tycker att du står lite bättre ({pawns})."),
        _ => format!("Jag tycker att du står klart bättre ({pawns})."),
    }
}

/// Hela kommentaren till motorns drag. `board` är ställningen före draget.
#[must_use]
pub fn comment(board: &Board, choice: &Choice) -> String {
    let sv = |mv| to_swedish(&san(board, mv));
    let mut parts = vec![format!("Jag spelar {}.", sv(choice.mv))];
    let found = ideas(board, choice.mv);
    if found.is_empty() {
        parts.push("Ett lugnt drag som förbättrar min ställning.".to_owned());
    }
    parts.extend(found.into_iter().map(sentence));
    parts.push(assessment(choice.score));

    let others: Vec<String> = choice
        .scored
        .iter()
        .filter(|(mv, _)| *mv != choice.mv)
        .take(ALTERNATIVES)
        .map(|(mv, _)| sv(*mv))
        .collect();
    if !others.is_empty() {
        parts.push(format!("Jag övervägde också {}.", others.join(" och ")));
    }

    if let Some((best, best_score)) = choice.scored.first()
        && best_score.saturating_sub(choice.score) >= ADMIT_WORSE
    {
        parts.push(format!(
            "Det bästa jag såg var {}, men på min nivå väljer jag inte alltid det bästa. Kan du straffa det?",
            sv(*best)
        ));
    }
    parts.join(" ")
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    clippy::shadow_unrelated,
    reason = "testnamnen är dokumentationen och fen återanvänds per testfall"
)]
mod tests {
    use super::{Board, Choice, Idea, MATE, Piece, assessment, capitalise, comment, sentence};
    use chess_trainer_engine::Square;

    fn setup(fen: &str, chosen: &str, scored: &[(&str, i32)]) -> (Board, Choice) {
        let board: Board = fen.parse().unwrap();
        let scored: Vec<_> = scored
            .iter()
            .map(|(mv, s)| (mv.parse().unwrap(), *s))
            .collect();
        let mv = chosen.parse().unwrap();
        let score = scored.iter().find(|(m, _)| *m == mv).unwrap().1;
        (board, Choice { mv, score, scored })
    }

    #[test]
    fn sentences_for_every_idea() {
        assert_eq!(sentence(Idea::Mate), "Det är schack matt!");
        assert_eq!(sentence(Idea::Check), "Det ger schack.");
        assert!(sentence(Idea::Castles).starts_with("Jag rockerar"));
        assert_eq!(sentence(Idea::Captures(Piece::Rook)), "Jag slår ditt torn.");
        assert_eq!(sentence(Idea::Captures(Piece::Pawn)), "Jag slår din bonde.");
        assert_eq!(
            sentence(Idea::Escapes(Piece::Knight)),
            "Min springare var hotad, så jag flyttar undan den."
        );
        assert_eq!(
            sentence(Idea::Escapes(Piece::Rook)),
            "Mitt torn var hotat, så jag flyttar undan den."
        );
        assert_eq!(
            sentence(Idea::Threatens(Piece::Queen, Square::D1)),
            "Nu hotar jag din dam på d1."
        );
        assert_eq!(
            sentence(Idea::Develops(Piece::Bishop)),
            "Jag utvecklar min löpare mot mitten."
        );
        assert_eq!(sentence(Idea::TakesCenter), "Bonden tar plats i centrum.");
        assert_eq!(sentence(Idea::Captures(Piece::King)), "Jag slår din kung.");
    }

    #[test]
    fn capitalise_handles_empty_text() {
        assert_eq!(capitalise(""), "");
        assert_eq!(capitalise("åsna"), "Åsna");
    }

    #[test]
    fn assessment_scale() {
        assert_eq!(assessment(MATE - 3), "Jag ser en väg till matt.");
        assert_eq!(
            assessment(3 - MATE),
            "Jag ser att du kan göra matt. Hittar du det?"
        );
        assert_eq!(
            assessment(MATE / 2),
            "Jag tycker att jag står klart bättre (+500,0)."
        );
        assert_eq!(
            assessment(-(MATE / 2)),
            "Jag tycker att du står klart bättre (-500,0)."
        );
        assert_eq!(
            assessment(150),
            "Jag tycker att jag står klart bättre (+1,5)."
        );
        assert_eq!(
            assessment(149),
            "Jag tycker att jag står lite bättre (+1,5)."
        );
        assert_eq!(
            assessment(50),
            "Jag tycker att jag står lite bättre (+0,5)."
        );
        assert_eq!(assessment(49), "Ställningen känns jämn (+0,5).");
        assert_eq!(assessment(0), "Ställningen känns jämn (+0,0).");
        assert_eq!(assessment(-49), "Ställningen känns jämn (-0,5).");
        assert_eq!(
            assessment(-50),
            "Jag tycker att du står lite bättre (-0,5)."
        );
        assert_eq!(
            assessment(-149),
            "Jag tycker att du står lite bättre (-1,5)."
        );
        assert_eq!(
            assessment(-150),
            "Jag tycker att du står klart bättre (-1,5)."
        );
    }

    #[test]
    fn full_comment_with_alternatives() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
        let (board, choice) = setup(fen, "b8c6", &[("b8c6", 10), ("g8f6", 5), ("a7a6", -5)]);
        assert_eq!(
            comment(&board, &choice),
            "Jag spelar Sc6. Jag utvecklar min springare mot mitten. Ställningen känns jämn (+0,1). \
             Jag övervägde också Sf6 och a6."
        );
    }

    #[test]
    fn quiet_move_and_admitting_a_worse_choice() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
        let (board, choice) = setup(fen, "a7a6", &[("g8f6", 40), ("b8c6", 30), ("a7a6", 10)]);
        assert_eq!(
            comment(&board, &choice),
            "Jag spelar a6. Ett lugnt drag som förbättrar min ställning. Ställningen känns jämn (+0,1). \
             Jag övervägde också Sf6 och Sc6. Det bästa jag såg var Sf6, men på min nivå väljer jag \
             inte alltid det bästa. Kan du straffa det?"
        );
    }

    #[test]
    fn small_differences_are_not_admitted() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
        let (board, choice) = setup(fen, "a7a6", &[("g8f6", 39), ("a7a6", 10)]);
        assert!(!comment(&board, &choice).contains("bästa jag såg"));
        let (board, choice) = setup(fen, "a7a6", &[("g8f6", 40), ("a7a6", 10)]);
        assert!(comment(&board, &choice).contains("bästa jag såg var Sf6"));
    }

    #[test]
    fn single_and_no_alternatives() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
        let (board, choice) = setup(fen, "a7a6", &[("a7a6", 0), ("g8f6", 0)]);
        assert!(comment(&board, &choice).ends_with("Jag övervägde också Sf6."));
        let (board, choice) = setup(fen, "a7a6", &[("a7a6", 0)]);
        assert!(comment(&board, &choice).ends_with("Ställningen känns jämn (+0,0)."));
    }
}
