//! Bedömer spelarens drag: var det ett tydligt fel, och i så fall vilken sort?
//!
//! Tränaren bryr sig om de enkla felen som chatten handlade om: hängande pjäser,
//! missade slag och missade hot.

use cozy_chess::{Board, Move};
use serde::{Deserialize, Serialize};

use crate::san::{is_capture, san};
use crate::search::score_moves;

/// Sökdjup för bedömningen. Två halvdrag plus slagsökning räcker för enkla fel.
const DEPTH: u8 = 2;

/// Så många centibönder sämre än bästa draget räknas som ett fel. Ungefär en och en halv bonde.
// ponytail: fast tröskel; skala med spelarens rating om feedbacken blir för tjatig eller för snäll.
pub const THRESHOLD: i32 = 150;

/// Räknas en förlust på `loss` centibönder som ett fel?
#[must_use]
pub const fn is_mistake(loss: i32) -> bool {
    loss >= THRESHOLD
}

/// Typ av fel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MistakeKind {
    /// Pjäsen kunde slås utan att spelaren fick tillbaka lika mycket.
    HangingPiece,
    /// Det fanns ett slag som vann material, men spelaren tog det inte.
    MissedCapture,
    /// Annat drag som tappade material, oftast ett missat hot.
    MissedThreat,
}

/// Ett fel spelaren gjorde.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mistake {
    /// Det spelade draget i SAN.
    pub played: String,
    /// Motorns bästa drag i SAN.
    pub best: String,
    /// Hur många centibönder sämre det spelade draget var.
    pub loss: i32,
    /// Vilken sorts fel.
    pub kind: MistakeKind,
}

/// Bästa draget och dess värde ur den dragandes perspektiv.
fn best(scored: &[(Move, i32)]) -> Option<(Move, i32)> {
    scored.iter().copied().max_by_key(|(_, score)| *score)
}

/// Bedömer draget `played` i ställningen `board`. `None` om draget var godtagbart eller olagligt.
#[must_use]
pub fn assess(board: &Board, played: Move) -> Option<Mistake> {
    let scored = score_moves(board, DEPTH);
    let (best_move, best_score) = best(&scored)?;
    let played_score = scored.iter().find(|(mv, _)| *mv == played)?.1;
    let loss = best_score.saturating_sub(played_score);
    if !is_mistake(loss) {
        return None;
    }

    let mut after = board.clone();
    after.play_unchecked(played);
    let reply = best(&score_moves(&after, 1)).map(|(mv, _)| mv);
    let kind = if is_capture(board, best_move) && !is_capture(board, played) {
        MistakeKind::MissedCapture
    } else if reply.is_some_and(|mv| mv.to == played.to && is_capture(&after, mv)) {
        MistakeKind::HangingPiece
    } else {
        MistakeKind::MissedThreat
    };

    Some(Mistake {
        played: san(board, played),
        best: san(board, best_move),
        loss,
        kind,
    })
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    clippy::shadow_unrelated,
    reason = "testnamnen är dokumentationen och fen återanvänds per testfall"
)]
mod tests {
    use super::{Board, MistakeKind, THRESHOLD, assess, is_mistake};

    fn assess_fen(fen: &str, uci: &str) -> Option<super::Mistake> {
        let board: Board = fen.parse().unwrap();
        assess(&board, uci.parse().unwrap())
    }

    #[test]
    fn threshold_is_one_and_a_half_pawns() {
        assert_eq!(THRESHOLD, 150);
        assert!(is_mistake(150));
        assert!(!is_mistake(149));
    }

    #[test]
    fn good_move_is_not_a_mistake() {
        assert_eq!(
            assess_fen(
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                "e2e4"
            ),
            None
        );
    }

    #[test]
    fn illegal_move_is_ignored() {
        assert_eq!(
            assess_fen(
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                "e2e5"
            ),
            None
        );
    }

    #[test]
    fn no_moves_means_no_assessment() {
        assert_eq!(
            assess_fen("R5k1/5ppp/8/8/8/8/8/4K3 b - - 0 1", "g8h8"),
            None
        );
    }

    #[test]
    fn hanging_the_queen() {
        // Dd1-d5 ställer damen där e6-bonden slår den.
        let mistake = assess_fen("4k3/8/4p3/8/8/8/8/3QK3 w - - 0 1", "d1d5").unwrap();
        assert_eq!(mistake.kind, MistakeKind::HangingPiece);
        assert_eq!(mistake.played, "Qd5");
        assert!(mistake.loss >= 800, "{}", mistake.loss);
    }

    #[test]
    fn missing_a_free_rook() {
        // Svart torn på d5 hänger, men vit spelar kungen.
        let mistake = assess_fen("4k3/8/8/3r4/8/8/8/3QK3 w - - 0 1", "e1f1").unwrap();
        assert_eq!(mistake.kind, MistakeKind::MissedCapture);
        assert_eq!(mistake.best, "Qxd5");
        assert_eq!(mistake.played, "Kf1");
    }

    #[test]
    fn capturing_the_wrong_piece_while_something_else_hangs() {
        // Vit slår h7-bonden med tornet när tornet på a5 hänger och hotar damen.
        // Båda dragen är slag, så det räknas inte som ett missat slag.
        let mistake = assess_fen("4k3/7p/8/r7/8/8/8/Q3K2R w - - 0 1", "h1h7").unwrap();
        assert_eq!(mistake.kind, MistakeKind::MissedThreat);
        assert_eq!(mistake.played, "Rxh7");
    }

    #[test]
    fn ignoring_a_threat() {
        // Svart hotar Txd1. Vit flyttar en bonde i stället för att rädda damen.
        let mistake = assess_fen("3rk3/8/8/8/8/8/P7/3QK3 w - - 0 1", "a2a3").unwrap();
        assert_eq!(mistake.kind, MistakeKind::MissedThreat);
        assert_eq!(mistake.played, "a3");
    }
}
