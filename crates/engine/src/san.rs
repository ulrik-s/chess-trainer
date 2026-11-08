//! Standard algebraisk notation (SAN) och omvandling från rutor till drag.
//!
//! `cozy-chess` kodar rockad som att kungen "slår" sitt eget torn. Den här
//! modulen döljer det: utåt är rockad `O-O`/`O-O-O` och kungen landar på g- eller c-linjen.

use cozy_chess::{Board, File, GameStatus, Move, Piece, Square};

/// Alla lagliga drag i ställningen.
#[must_use]
pub fn legal_moves(board: &Board) -> Vec<Move> {
    let mut moves = Vec::new();
    board.generate_moves(|piece_moves| {
        moves.extend(piece_moves);
        false
    });
    moves
}

/// SAN-bokstav för en pjäs. Bonden har ingen.
pub(crate) const fn letter(piece: Piece) -> &'static str {
    match piece {
        Piece::Pawn => "",
        Piece::Knight => "N",
        Piece::Bishop => "B",
        Piece::Rook => "R",
        Piece::Queen => "Q",
        Piece::King => "K",
    }
}

/// Är rockaden kort? Tornet står då på h-linjen. Bara vanligt schack, inte Chess960.
fn is_short_castle(mv: Move) -> bool {
    mv.to.file() == File::H
}

/// Är draget en rockad, dvs. flyttar kungen till en ruta med ett eget torn?
pub(crate) fn is_castle(board: &Board, mv: Move) -> bool {
    board.piece_on(mv.from) == Some(Piece::King)
        && board.color_on(mv.to) == Some(board.side_to_move())
}

/// Rutan där pjäsen landar så som en människa ser det. Vid rockad är det kungens slutruta.
#[must_use]
pub fn landing_square(board: &Board, mv: Move) -> Square {
    if is_castle(board, mv) {
        let file = if is_short_castle(mv) {
            File::G
        } else {
            File::C
        };
        Square::new(file, mv.from.rank())
    } else {
        mv.to
    }
}

/// Slår draget en pjäs? En passant räknas som slag.
pub(crate) fn is_capture(board: &Board, mv: Move) -> bool {
    let en_passant = board.piece_on(mv.from) == Some(Piece::Pawn) && mv.from.file() != mv.to.file();
    en_passant || board.color_on(mv.to) == Some(!board.side_to_move())
}

/// Kolumnens bokstav, som `e`.
fn file_char(square: Square) -> char {
    square.to_string().chars().next().unwrap_or('a')
}

/// Radens siffra, som `4`.
fn rank_char(square: Square) -> char {
    square.to_string().chars().nth(1).unwrap_or('1')
}

/// Särskiljare när flera pjäser av samma slag kan nå samma ruta.
fn disambiguation(board: &Board, mv: Move, piece: Piece) -> String {
    let rivals: Vec<Square> = legal_moves(board)
        .into_iter()
        .filter(|other| {
            other.to == mv.to && other.from != mv.from && board.piece_on(other.from) == Some(piece)
        })
        .map(|other| other.from)
        .collect();
    if rivals.is_empty() {
        String::new()
    } else if rivals.iter().all(|sq| sq.file() != mv.from.file()) {
        file_char(mv.from).to_string()
    } else if rivals.iter().all(|sq| sq.rank() != mv.from.rank()) {
        rank_char(mv.from).to_string()
    } else {
        mv.from.to_string()
    }
}

/// SAN utan schack- eller mattecken.
fn san_body(board: &Board, mv: Move) -> String {
    if is_castle(board, mv) {
        return if is_short_castle(mv) { "O-O" } else { "O-O-O" }.to_owned();
    }
    let piece = board.piece_on(mv.from).unwrap_or(Piece::Pawn);
    let capture = is_capture(board, mv);
    let mut out = String::from(letter(piece));
    if piece == Piece::Pawn {
        if capture {
            out.push(file_char(mv.from));
        }
    } else {
        out.push_str(&disambiguation(board, mv, piece));
    }
    if capture {
        out.push('x');
    }
    out.push_str(&mv.to.to_string());
    if let Some(promotion) = mv.promotion {
        out.push('=');
        out.push_str(letter(promotion));
    }
    out
}

/// Draget i SAN, med `+` vid schack och `#` vid matt.
#[must_use]
pub fn san(board: &Board, mv: Move) -> String {
    let mut out = san_body(board, mv);
    let mut after = board.clone();
    after.play_unchecked(mv);
    if after.status() == GameStatus::Won {
        out.push('#');
    } else if !after.checkers().is_empty() {
        out.push('+');
    }
    out
}

/// Hittar det lagliga drag som har given SAN. Schack- och mattecken är valfria.
#[must_use]
pub fn parse_san(board: &Board, input: &str) -> Option<Move> {
    let wanted = input.trim_end_matches(['+', '#']);
    legal_moves(board)
        .into_iter()
        .find(|mv| san_body(board, *mv) == wanted)
}

/// Hittar draget från `from` till `to`. Bönder som når sista raden blir damer.
#[must_use]
pub fn from_squares(board: &Board, from: Square, to: Square) -> Option<Move> {
    legal_moves(board).into_iter().find(|mv| {
        mv.from == from
            && landing_square(board, *mv) == to
            && matches!(mv.promotion, None | Some(Piece::Queen))
    })
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    clippy::shadow_unrelated,
    reason = "testnamnen är dokumentationen och fen återanvänds per testfall"
)]
mod tests {
    use super::{Board, Piece, Square, from_squares, landing_square, legal_moves, parse_san, san};

    fn board(fen: &str) -> Board {
        fen.parse().unwrap()
    }

    fn san_of(fen: &str, uci: &str) -> String {
        let b = board(fen);
        san(&b, uci.parse().unwrap())
    }

    #[test]
    fn startpos_has_twenty_moves() {
        assert_eq!(legal_moves(&Board::default()).len(), 20);
    }

    #[test]
    fn pawn_and_piece_moves() {
        let start = Board::default();
        assert_eq!(san(&start, "e2e4".parse().unwrap()), "e4");
        assert_eq!(san(&start, "g1f3".parse().unwrap()), "Nf3");
    }

    #[test]
    fn captures_include_x_and_pawn_file() {
        let fen = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        assert_eq!(san_of(fen, "e4d5"), "exd5");
        let fen = "rnbqkbnr/ppp1pppp/8/3p4/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 2";
        assert_eq!(san_of(fen, "e4d5"), "exd5");
    }

    #[test]
    fn bishop_move() {
        let fen = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        assert_eq!(san_of(fen, "f1c4"), "Bc4");
    }

    #[test]
    fn piece_capture() {
        let fen = "rnbqkbnr/pppp1ppp/8/4p3/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 2";
        assert_eq!(san_of(fen, "f3e5"), "Nxe5");
    }

    #[test]
    fn en_passant_is_a_capture() {
        let fen = "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3";
        assert_eq!(san_of(fen, "e5f6"), "exf6");
    }

    #[test]
    fn castling_both_sides() {
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        assert_eq!(san_of(fen, "e1h1"), "O-O");
        assert_eq!(san_of(fen, "e1a1"), "O-O-O");
    }

    #[test]
    fn check_and_mate_suffixes() {
        let fen = "4k3/8/8/8/8/8/8/R3K3 w - - 0 1";
        assert_eq!(san_of(fen, "a1a8"), "Ra8+");
        let fen = "6k1/5ppp/8/8/8/8/8/R3K3 w - - 0 1";
        assert_eq!(san_of(fen, "a1a8"), "Ra8#");
    }

    #[test]
    fn promotion() {
        let fen = "8/4P3/8/8/8/8/k7/4K3 w - - 0 1";
        assert_eq!(san_of(fen, "e7e8q"), "e8=Q");
        assert_eq!(san_of(fen, "e7e8n"), "e8=N");
    }

    #[test]
    fn disambiguation_by_file_rank_and_square() {
        // Två springare på b1 och f1 kan båda nå d2: kolumn räcker.
        let fen = "4k3/8/8/8/8/8/8/1N2KN2 w - - 0 1";
        assert_eq!(san_of(fen, "b1d2"), "Nbd2");
        // Två torn på a1 och a5 kan båda nå a3: rad räcker.
        let fen = "4k3/8/8/R7/8/8/8/R3K3 w - - 0 1";
        assert_eq!(san_of(fen, "a1a3"), "R1a3");
        // Tre damer: a1, a3 och c1 når alla b2; a1 delar kolumn med a3 och rad med c1.
        let fen = "4k3/8/8/8/8/Q7/8/Q1Q1K3 w - - 0 1";
        assert_eq!(san_of(fen, "a1b2"), "Qa1b2");
    }

    #[test]
    fn piece_that_is_not_ambiguous_has_no_disambiguation() {
        // Springaren på g1 kan inte nå d2, så b1-springaren behöver ingen särskiljare.
        let fen = "4k3/8/8/8/8/8/8/1N2K1N1 w - - 0 1";
        assert_eq!(san_of(fen, "b1d2"), "Nd2");
    }

    #[test]
    fn parse_san_finds_moves_and_ignores_check_marks() {
        let start = Board::default();
        assert_eq!(parse_san(&start, "Nf3"), Some("g1f3".parse().unwrap()));
        let fen = "4k3/8/8/8/8/8/8/R3K3 w - - 0 1";
        let b = board(fen);
        assert_eq!(parse_san(&b, "Ra8"), Some("a1a8".parse().unwrap()));
        assert_eq!(parse_san(&b, "Ra8+"), Some("a1a8".parse().unwrap()));
        assert_eq!(parse_san(&b, "Ra8#"), Some("a1a8".parse().unwrap()));
    }

    #[test]
    fn parse_san_rejects_illegal_moves() {
        assert_eq!(parse_san(&Board::default(), "e5"), None);
    }

    #[test]
    fn from_squares_handles_castling_and_promotion() {
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        let b = board(fen);
        assert_eq!(
            from_squares(&b, Square::E1, Square::G1),
            Some("e1h1".parse().unwrap())
        );
        assert_eq!(
            from_squares(&b, Square::E1, Square::C1),
            Some("e1a1".parse().unwrap())
        );
        assert_eq!(
            from_squares(&b, Square::E1, Square::F1),
            Some("e1f1".parse().unwrap())
        );
        let fen = "8/4P3/8/8/8/8/k7/4K3 w - - 0 1";
        let promoted = from_squares(&board(fen), Square::E7, Square::E8).unwrap();
        assert_eq!(promoted.promotion, Some(Piece::Queen));
        assert_eq!(
            from_squares(&Board::default(), Square::E2, Square::E5),
            None
        );
        assert_eq!(
            from_squares(&Board::default(), Square::D2, Square::E4),
            None
        );
    }

    #[test]
    fn landing_square_of_normal_move_is_its_target() {
        let b = Board::default();
        assert_eq!(landing_square(&b, "e2e4".parse().unwrap()), Square::E4);
    }
}
