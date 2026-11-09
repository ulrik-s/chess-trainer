//! Sökning: negamax med alfa-beta och en enkel slagsökning i lövnoderna.
//!
//! Styrkan sätts med sökdjup och "brus": motorn väljer slumpvis bland alla drag
//! som är högst `noise` centibönder sämre än det bästa.

use cozy_chess::{Board, Color, Move, Piece, Rank, Square};

use crate::san::{is_capture, legal_moves};

/// Poäng för matt. Större än all material tillsammans.
pub const MATE: i32 = 100_000;

/// Sökfönstrets gräns. Större än alla mattvärden, som kan vara `MATE` plus återstående djup.
const INF: i32 = i32::MAX;

/// Materialvärde i centibönder.
#[must_use]
pub const fn value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 100,
        Piece::Knight | Piece::Bishop => 300,
        Piece::Rook => 500,
        Piece::Queen => 900,
        Piece::King => 0,
    }
}

/// Bonus för pjäser i centrum, så att motorn inte spelar planlöst.
const CENTER_BONUS: i32 = 10;

/// De fyra centrumrutorna.
const CENTER: [Square; 4] = [Square::D4, Square::E4, Square::D5, Square::E5];

/// Avdrag per springare eller löpare som står kvar på första raden. Får motorn, och tipsen,
/// att utveckla pjäser i stället för att flytta bönder planlöst.
// ponytail: ett enda positionellt mått; piece-square-tabeller om spelet behöver bli starkare.
const UNDEVELOPED_PENALTY: i32 = 15;

/// Ställningens värde för en färg: material, centrum och utveckling.
fn side_score(board: &Board, color: Color) -> i32 {
    let ours = board.colors(color);
    let material = Piece::ALL.iter().fold(0_i32, |sum, &piece| {
        let count = i32::try_from((board.pieces(piece) & ours).len()).unwrap_or(0);
        sum.saturating_add(value(piece).saturating_mul(count))
    });
    let center = CENTER.iter().filter(|sq| ours.has(**sq)).count();
    let back_rank = Rank::First.relative_to(color).bitboard() & ours;
    let undeveloped = (board.pieces(Piece::Knight) & back_rank)
        .len()
        .saturating_add((board.pieces(Piece::Bishop) & back_rank).len());
    material
        .saturating_add(CENTER_BONUS.saturating_mul(i32::try_from(center).unwrap_or(0)))
        .saturating_sub(UNDEVELOPED_PENALTY.saturating_mul(i32::try_from(undeveloped).unwrap_or(0)))
}

/// Statisk värdering ur den dragande sidans perspektiv.
#[must_use]
pub fn evaluate(board: &Board) -> i32 {
    let us = board.side_to_move();
    side_score(board, us).saturating_sub(side_score(board, !us))
}

/// Lagliga drag med slag först, mest värdefulla offer först. Gör att alfa-beta skär mer.
fn ordered_moves(board: &Board) -> Vec<Move> {
    let mut moves = legal_moves(board);
    moves.sort_by_key(|mv| {
        board
            .piece_on(mv.to)
            .map_or(0, |victim| value(victim).saturating_neg())
    });
    moves
}

/// Högst så många slag i följd söks i slagsökningen.
const QUIESCENCE_DEPTH: u8 = 8;

/// Slagsökning: fortsätter bara med slag tills ställningen är lugn.
/// Utan den tror motorn att den vinner en pjäs som slås tillbaka nästa drag.
/// `depth_left` begränsar hur många slag i följd som söks, så att sökningen alltid tar slut.
fn quiescence(board: &Board, mut alpha: i32, beta: i32, depth_left: u8) -> i32 {
    let stand_pat = evaluate(board);
    if stand_pat >= beta {
        return stand_pat;
    }
    let Some(next_depth) = depth_left.checked_sub(1) else {
        return stand_pat;
    };
    alpha = alpha.max(stand_pat);
    for mv in ordered_moves(board)
        .into_iter()
        .filter(|mv| is_capture(board, *mv))
    {
        let mut child = board.clone();
        child.play_unchecked(mv);
        let score = quiescence(
            &child,
            beta.saturating_neg(),
            alpha.saturating_neg(),
            next_depth,
        )
        .saturating_neg();
        if score >= beta {
            return score;
        }
        alpha = alpha.max(score);
    }
    alpha
}

/// Negamax med alfa-beta. Matt med mer återstående djup ligger närmare roten och värderas högre.
fn negamax(board: &Board, depth: u8, mut alpha: i32, beta: i32) -> i32 {
    let moves = ordered_moves(board);
    if moves.is_empty() {
        return if board.checkers().is_empty() {
            0
        } else {
            MATE.saturating_add(i32::from(depth)).saturating_neg()
        };
    }
    if depth == 0 {
        return quiescence(board, alpha, beta, QUIESCENCE_DEPTH);
    }
    let mut best = INF.saturating_neg();
    for mv in moves {
        let mut child = board.clone();
        child.play_unchecked(mv);
        let score = negamax(
            &child,
            depth.saturating_sub(1),
            beta.saturating_neg(),
            alpha.saturating_neg(),
        )
        .saturating_neg();
        best = best.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }
    best
}

/// Värdet av varje lagligt drag ur den dragandes perspektiv, sökt `depth` halvdrag.
#[must_use]
pub fn score_moves(board: &Board, depth: u8) -> Vec<(Move, i32)> {
    legal_moves(board)
        .into_iter()
        .map(|mv| {
            let mut child = board.clone();
            child.play_unchecked(mv);
            let score = negamax(&child, depth.saturating_sub(1), INF.saturating_neg(), INF)
                .saturating_neg();
            (mv, score)
        })
        .collect()
}

/// Motorns styrka.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strength {
    /// Sökdjup i halvdrag, minst 1.
    pub depth: u8,
    /// Hur många centibönder sämre än bästa draget motorn kan välja.
    pub noise: i32,
}

/// Liten deterministisk slumpgenerator (xorshift64). Fröet styr hela partiet, så tester blir repeterbara.
// ponytail: egen xorshift i stället för rand-craten; byt om bättre slump någonsin behövs.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// Ny generator. Fröet 0 ersätts, eftersom xorshift fastnar på 0.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    /// Nästa slumptal.
    pub const fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Slumpmässigt index i `0..len`. `len` måste vara större än 0.
    fn index(&mut self, len: usize) -> usize {
        let len64 = u64::try_from(len).unwrap_or(u64::MAX);
        usize::try_from(self.next_u64().checked_rem(len64).unwrap_or(0)).unwrap_or(0)
    }
}

/// Motorns val: draget och vad den tyckte om alla alternativ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// Det valda draget.
    pub mv: Move,
    /// Värdet av det valda draget, ur motorns perspektiv.
    pub score: i32,
    /// Alla lagliga drag med sina värden, bästa först.
    pub scored: Vec<(Move, i32)>,
}

/// Väljer motorns drag. `None` om det inte finns något lagligt drag.
#[must_use]
pub fn choose(board: &Board, strength: Strength, rng: &mut Rng) -> Option<Choice> {
    let mut scored = score_moves(board, strength.depth.max(1));
    scored.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    let best = scored.first()?.1;
    let floor = best.saturating_sub(strength.noise);
    let candidates: Vec<(Move, i32)> = scored
        .iter()
        .copied()
        .filter(|(_, score)| *score >= floor)
        .collect();
    // Bästa draget finns alltid bland kandidaterna, så listan är aldrig tom.
    candidates
        .get(rng.index(candidates.len()))
        .map(|&(mv, score)| Choice { mv, score, scored })
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    clippy::shadow_unrelated,
    reason = "testnamnen är dokumentationen och fen återanvänds per testfall"
)]
mod tests {
    use super::{
        Board, MATE, Piece, Rng, Strength, choose, evaluate, quiescence, score_moves, value,
    };

    fn board(fen: &str) -> Board {
        fen.parse().unwrap()
    }

    #[test]
    fn piece_values() {
        assert_eq!(value(Piece::Pawn), 100);
        assert_eq!(value(Piece::Knight), 300);
        assert_eq!(value(Piece::Bishop), 300);
        assert_eq!(value(Piece::Rook), 500);
        assert_eq!(value(Piece::Queen), 900);
        assert_eq!(value(Piece::King), 0);
    }

    #[test]
    fn startpos_is_balanced() {
        assert_eq!(evaluate(&Board::default()), 0);
    }

    #[test]
    fn evaluation_is_from_side_to_move() {
        // Vit har en extra dam.
        let white_to_move = board("4k3/8/8/8/8/8/8/3QK3 w - - 0 1");
        assert_eq!(evaluate(&white_to_move), 900);
        let black_to_move = board("4k3/8/8/8/8/8/8/3QK3 b - - 0 1");
        assert_eq!(evaluate(&black_to_move), -900);
    }

    #[test]
    fn center_pieces_get_a_bonus() {
        let fen = "4k3/8/8/8/4P3/8/8/4K3 w - - 0 1";
        assert_eq!(evaluate(&board(fen)), 110);
        let fen = "4k3/8/8/3PP3/3PP3/8/8/4K3 w - - 0 1";
        assert_eq!(evaluate(&board(fen)), 440);
    }

    #[test]
    fn undeveloped_minor_pieces_cost_points() {
        // Vit springare kvar på g1, svart springare utvecklad till f6.
        let fen = "4k3/8/5n2/8/8/8/8/4K1N1 w - - 0 1";
        assert_eq!(evaluate(&board(fen)), -15);
        // Vit löpare på c1 och svart löpare på c8 tar ut varandra.
        let fen = "2b1k3/8/8/8/8/8/8/2B1K3 w - - 0 1";
        assert_eq!(evaluate(&board(fen)), 0);
        // En vit löpare på åttonde raden räknas inte som outvecklad.
        let fen = "B3k3/8/8/8/8/8/8/4K3 w - - 0 1";
        assert_eq!(evaluate(&board(fen)), 300);
    }

    #[test]
    fn engine_prefers_developing_to_pawn_moves() {
        let mv = choose(
            &Board::default(),
            Strength { depth: 2, noise: 0 },
            &mut Rng::new(1),
        )
        .unwrap()
        .mv;
        // En springare ut ger mer än ett bondedrag, eftersom den inte längre står outvecklad.
        assert_eq!(Board::default().piece_on(mv.from), Some(Piece::Knight));
    }

    #[test]
    fn quiescence_sees_recapture() {
        // Vit dam kan slå en bonde på d5 som är skyddad av en bonde på e6: lugnt läge är bättre.
        let fen = "4k3/8/4p3/3p4/8/8/8/3QK3 w - - 0 1";
        let b = board(fen);
        let quiet = evaluate(&b);
        assert_eq!(quiescence(&b, -MATE, MATE, 8), quiet);
    }

    #[test]
    fn quiescence_takes_free_material() {
        // Svart torn på d5 är oskyddat.
        let fen = "4k3/8/8/3r4/8/8/8/3QK3 w - - 0 1";
        assert_eq!(quiescence(&board(fen), -MATE, MATE, 8), 910);
    }

    #[test]
    fn quiescence_stops_at_its_depth_limit() {
        // Utan djup kvar räknas bara stå-still-värdet, trots att tornet hänger.
        let fen = "4k3/8/8/3r4/8/8/8/3QK3 w - - 0 1";
        assert_eq!(quiescence(&board(fen), -MATE, MATE, 0), 390);
        // Ett slag räcker för att ta tornet.
        assert_eq!(quiescence(&board(fen), -MATE, MATE, 1), 910);
        assert_eq!(super::QUIESCENCE_DEPTH, 8);
    }

    #[test]
    fn quiescence_beta_cutoffs() {
        let fen = "4k3/8/8/3r4/8/8/8/3QK3 w - - 0 1";
        let b = board(fen);
        // Stå-still-värdet (390, tornet står i centrum) är redan över beta.
        assert_eq!(quiescence(&b, -MATE, 100, 8), 390);
        // Slaget går över beta men inte stå-still-värdet. Vid avskärning blir svaret en gräns (beta), inte exakt värde.
        assert_eq!(quiescence(&b, -MATE, 500, 8), 500);
    }

    #[test]
    fn quiescence_keeps_looking_after_a_bad_capture() {
        // Dxd5 (tornet först i ordningen) förlorar damen; Dxa4 vinner en springare gratis.
        let fen = "4k3/8/4p3/3r4/n7/8/8/3QK3 w - - 0 1";
        assert_eq!(quiescence(&board(fen), -MATE, MATE, 8), 290);
    }

    #[test]
    fn finds_mate_in_one() {
        let fen = "6k1/5ppp/8/8/8/8/8/R3K3 w - - 0 1";
        let b = board(fen);
        let best = score_moves(&b, 1)
            .into_iter()
            .max_by_key(|(_, s)| *s)
            .unwrap();
        assert_eq!(best.0, "a1a8".parse().unwrap());
        assert_eq!(best.1, MATE);
    }

    #[test]
    fn stalemate_scores_zero() {
        // Svart kung på h8 är patt efter Dg6.
        let fen = "7k/8/8/5Q2/8/8/8/K7 w - - 0 1";
        let b = board(fen);
        let scored = score_moves(&b, 1);
        let stalemate = scored
            .iter()
            .find(|(mv, _)| *mv == "f5g6".parse().unwrap())
            .unwrap();
        assert_eq!(stalemate.1, 0);
    }

    #[test]
    fn prefers_faster_mate() {
        // Matt i ett finns; djup 3 ska föredra det framför matt längre fram.
        let fen = "6k1/5ppp/8/8/8/8/8/R3K3 w - - 0 1";
        let b = board(fen);
        let best = score_moves(&b, 3)
            .into_iter()
            .max_by_key(|(_, s)| *s)
            .unwrap();
        assert_eq!(best.0, "a1a8".parse().unwrap());
        assert_eq!(best.1, MATE + 2);
    }

    #[test]
    fn deeper_search_avoids_losing_the_queen() {
        // Dxd5 ser ut att vinna en bonde men damen slås av e6-bonden.
        let fen = "4k3/8/4p3/3p4/8/8/8/3QK3 w - - 0 1";
        let b = board(fen);
        let scored = score_moves(&b, 2);
        let grab = scored
            .iter()
            .find(|(mv, _)| *mv == "d1d5".parse().unwrap())
            .unwrap()
            .1;
        let best = scored.iter().map(|(_, s)| *s).max().unwrap();
        assert_eq!(grab, -110);
        assert!(best - grab > 500, "{best} {grab}");
    }

    #[test]
    fn zero_noise_always_plays_best() {
        let fen = "4k3/8/8/3r4/8/8/8/3QK3 w - - 0 1";
        let b = board(fen);
        let strength = Strength { depth: 2, noise: 0 };
        for seed in 1..20 {
            let mv = choose(&b, strength, &mut Rng::new(seed)).unwrap().mv;
            assert_eq!(mv, "d1d5".parse().unwrap());
        }
    }

    #[test]
    fn noise_allows_other_moves() {
        let b = Board::default();
        let strength = Strength {
            depth: 1,
            noise: 1000,
        };
        let mut rng = Rng::new(7);
        let first = choose(&b, strength, &mut rng).unwrap().mv;
        let differs = (0..50).any(|_| choose(&b, strength, &mut rng).unwrap().mv != first);
        assert!(differs);
    }

    #[test]
    fn choice_reports_scores_best_first() {
        // Svart torn på d5 hänger: Dxd5 är bäst och står först.
        let fen = "4k3/8/8/3r4/8/8/8/3QK3 w - - 0 1";
        let choice = choose(
            &board(fen),
            Strength { depth: 2, noise: 0 },
            &mut Rng::new(1),
        )
        .unwrap();
        assert_eq!(choice.mv, "d1d5".parse().unwrap());
        assert_eq!(choice.scored[0], (choice.mv, choice.score));
        assert!(choice.scored.windows(2).all(|w| w[0].1 >= w[1].1));
        assert_eq!(choice.scored.len(), 17);
    }

    #[test]
    fn depth_zero_is_treated_as_one() {
        let fen = "6k1/5ppp/8/8/8/8/8/R3K3 w - - 0 1";
        let strength = Strength { depth: 0, noise: 0 };
        let mv = choose(&board(fen), strength, &mut Rng::new(1)).unwrap().mv;
        assert_eq!(mv, "a1a8".parse().unwrap());
    }

    #[test]
    fn no_move_when_game_is_over() {
        let mated = board("R5k1/5ppp/8/8/8/8/8/4K3 b - - 0 1");
        let strength = Strength { depth: 1, noise: 0 };
        assert_eq!(choose(&mated, strength, &mut Rng::new(1)), None);
    }

    #[test]
    fn rng_is_deterministic_and_avoids_zero_seed() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        assert_eq!(a.next_u64(), b.next_u64());
        let mut zero = Rng::new(0);
        assert_ne!(zero.next_u64(), 0);
        let mut one = Rng::new(1);
        assert_eq!(one.next_u64(), 1_082_269_761);
        assert_eq!(one.next_u64(), 1_152_992_998_833_853_505);
        assert_eq!(one.next_u64(), 11_177_516_664_432_764_457);
    }

    #[test]
    fn rng_index_stays_in_range() {
        let mut rng = Rng::new(3);
        assert!((0..100).all(|_| rng.index(5) < 5));
        assert_eq!(rng.index(1), 0);
    }
}
