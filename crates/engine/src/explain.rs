//! Vad ett drag gör, som råmaterial till motorns kommentarer.

use cozy_chess::{Board, GameStatus, Move, Piece, Rank, Square};

use crate::attacks::is_threatened;
use crate::san::{is_capture, is_castle, landing_square};

/// Högst så många hotade pjäser nämns per drag.
const MAX_THREATS: usize = 2;

/// De fyra centrumrutorna.
const CENTER: [Square; 4] = [Square::D4, Square::E4, Square::D5, Square::E5];

/// En sak som ett drag åstadkommer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Idea {
    /// Ger schack matt.
    Mate,
    /// Ger schack.
    Check,
    /// Rockerar.
    Castles,
    /// Slår en pjäs.
    Captures(Piece),
    /// Flyttar en pjäs som var hotad till en ruta där den inte är det.
    Escapes(Piece),
    /// Hotar en motståndarpjäs som inte var hotad före draget.
    Threatens(Piece, Square),
    /// Flyttar en springare eller löpare från första raden.
    Develops(Piece),
    /// Flyttar en bonde till centrum.
    TakesCenter,
}

/// Vad draget `mv` gör i ställningen `board`, viktigast först.
#[must_use]
pub fn ideas(board: &Board, mv: Move) -> Vec<Idea> {
    let us = board.side_to_move();
    let piece = board.piece_on(mv.from).unwrap_or(Piece::Pawn);
    let to = landing_square(board, mv);
    let mut after = board.clone();
    after.play_unchecked(mv);

    let capture = is_capture(board, mv);
    let mut found = Vec::new();
    if after.status() == GameStatus::Won {
        found.push(Idea::Mate);
    } else if !after.checkers().is_empty() {
        found.push(Idea::Check);
    }
    if is_castle(board, mv) {
        found.push(Idea::Castles);
    } else if capture {
        found.push(Idea::Captures(board.piece_on(mv.to).unwrap_or(Piece::Pawn)));
    }
    if !capture && is_threatened(board, mv.from, !us) && !is_threatened(&after, to, !us) {
        found.push(Idea::Escapes(piece));
    }
    let threats = after
        .colors(!us)
        .into_iter()
        .filter(|sq| is_threatened(&after, *sq, us) && !is_threatened(board, *sq, us))
        .take(MAX_THREATS)
        .map(|sq| Idea::Threatens(after.piece_on(sq).unwrap_or(Piece::Pawn), sq));
    found.extend(threats);
    if matches!(piece, Piece::Knight | Piece::Bishop)
        && mv.from.rank() == Rank::First.relative_to(us)
    {
        found.push(Idea::Develops(piece));
    }
    if piece == Piece::Pawn && CENTER.contains(&to) {
        found.push(Idea::TakesCenter);
    }
    found
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    clippy::shadow_unrelated,
    reason = "testnamnen är dokumentationen och fen återanvänds per testfall"
)]
mod tests {
    use super::{Board, Idea, Piece, Square, ideas};

    fn ideas_of(fen: &str, uci: &str) -> Vec<Idea> {
        let board: Board = fen.parse().unwrap();
        ideas(&board, uci.parse().unwrap())
    }

    #[test]
    fn center_pawn_and_development() {
        let start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        assert_eq!(ideas_of(start, "e2e4"), vec![Idea::TakesCenter]);
        assert_eq!(ideas_of(start, "g1f3"), vec![Idea::Develops(Piece::Knight)]);
        assert_eq!(ideas_of(start, "a2a3"), vec![]);
        // Svart utvecklar från sin första rad, rad 8.
        let black = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
        assert_eq!(ideas_of(black, "b8c6"), vec![Idea::Develops(Piece::Knight)]);
        // d5 tar centrum och hotar den oskyddade bonden på e4.
        assert_eq!(
            ideas_of(black, "d7d5"),
            vec![Idea::Threatens(Piece::Pawn, Square::E4), Idea::TakesCenter]
        );
    }

    #[test]
    fn bishop_developing_with_a_threat() {
        // Lb5 hotar springaren på c6, som saknar skydd.
        let fen = "r1bqkbnr/p1p1pppp/2n5/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 3";
        assert_eq!(
            ideas_of(fen, "f1b5"),
            vec![
                Idea::Threatens(Piece::Knight, Square::C6),
                Idea::Develops(Piece::Bishop)
            ]
        );
    }

    #[test]
    fn captures_check_and_mate() {
        // Dxd5 slår tornet som hotade damen; det räknas som slag, inte som flykt.
        let fen = "4k3/8/8/3r4/8/8/8/3QK3 w - - 0 1";
        assert_eq!(ideas_of(fen, "d1d5"), vec![Idea::Captures(Piece::Rook)]);
        let fen = "4k3/8/8/8/8/8/8/R3K3 w - - 0 1";
        assert_eq!(ideas_of(fen, "a1a8"), vec![Idea::Check]);
        let fen = "6k1/5ppp/8/8/8/8/8/R3K3 w - - 0 1";
        assert_eq!(ideas_of(fen, "a1a8"), vec![Idea::Mate]);
    }

    #[test]
    fn en_passant_captures_a_pawn() {
        let fen = "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3";
        assert!(ideas_of(fen, "e5f6").contains(&Idea::Captures(Piece::Pawn)));
    }

    #[test]
    fn castling_is_not_a_capture() {
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        assert_eq!(ideas_of(fen, "e1h1"), vec![Idea::Castles]);
    }

    #[test]
    fn escaping_a_threat() {
        // Springaren på d4 hotas av bonden på e5.
        let fen = "4k3/8/8/4p3/3N4/8/8/4K3 w - - 0 1";
        // Till b5 är den säker.
        assert_eq!(ideas_of(fen, "d4b5"), vec![Idea::Escapes(Piece::Knight)]);
        // Till f3 är den säker och hotar dessutom e5-bonden.
        assert_eq!(
            ideas_of(fen, "d4f3"),
            vec![
                Idea::Escapes(Piece::Knight),
                Idea::Threatens(Piece::Pawn, Square::E5)
            ]
        );
        // Kungen var aldrig hotad.
        assert_eq!(ideas_of(fen, "e1d1"), vec![]);
    }

    #[test]
    fn moving_into_another_threat_is_no_escape() {
        // Springaren på d4 hotas av e5-bonden. På c6 slår b7-bonden den.
        let fen = "4k3/1p6/8/4p3/3N4/8/8/4K3 w - - 0 1";
        assert!(!ideas_of(fen, "d4c6").contains(&Idea::Escapes(Piece::Knight)));
    }

    #[test]
    fn old_threats_are_not_repeated() {
        // Damen på d4 hotar redan tornet på a7; h3 skapar inget nytt.
        let fen = "4k3/r7/8/8/3Q4/8/7P/4K3 w - - 0 1";
        assert_eq!(ideas_of(fen, "h2h3"), vec![]);
    }

    #[test]
    fn at_most_two_threats_are_named() {
        // Dd4 hotar tornet på a7, springaren på g7 och tornet på h4. Bara två nämns.
        let fen = "4k3/r5n1/8/8/7r/8/8/3QK3 w - - 0 1";
        let found = ideas_of(fen, "d1d4");
        assert_eq!(
            found
                .iter()
                .filter(|i| matches!(i, Idea::Threatens(..)))
                .count(),
            2
        );
    }
}
