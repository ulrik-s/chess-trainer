//! Hur många pjäser som attackerar en ruta, för färgningen av brädet.

use cozy_chess::{
    Board, Color, Piece, Square, get_bishop_moves, get_king_moves, get_knight_moves,
    get_pawn_attacks, get_rook_moves,
};

/// Antal pjäser av färgen `color` som attackerar `square`.
///
/// Räknar varje pjäs som når rutan direkt. Bundna pjäser räknas, och pjäser bakom
/// varandra på samma linje (batterier) räknas bara en gång.
// ponytail: inga röntgenattacker; räkna batterier om färgningen visar sig missvisande.
#[must_use]
pub fn attackers(board: &Board, square: Square, color: Color) -> u32 {
    let occupied = board.occupied();
    let diagonal = get_bishop_moves(square, occupied);
    let straight = get_rook_moves(square, occupied);
    [
        // En bonde av `color` attackerar rutan om den står där en motståndarbonde på rutan skulle slå.
        (get_pawn_attacks(square, !color), Piece::Pawn),
        (get_knight_moves(square), Piece::Knight),
        (get_king_moves(square), Piece::King),
        (diagonal, Piece::Bishop),
        (diagonal, Piece::Queen),
        (straight, Piece::Rook),
        (straight, Piece::Queen),
    ]
    .into_iter()
    .map(|(reach, piece)| (reach & board.colored_pieces(color, piece)).len())
    .sum()
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    clippy::shadow_unrelated,
    reason = "testnamnen är dokumentationen och fen återanvänds per testfall"
)]
mod tests {
    use super::{Board, Color, Square, attackers};

    fn board(fen: &str) -> Board {
        fen.parse().unwrap()
    }

    #[test]
    fn startpos_pawns_and_knights() {
        let b = Board::default();
        // e3: bönderna på d2 och f2.
        assert_eq!(attackers(&b, Square::E3, Color::White), 2);
        // f3: bönderna på e2 och g2 samt springaren på g1.
        assert_eq!(attackers(&b, Square::F3, Color::White), 3);
        assert_eq!(attackers(&b, Square::E3, Color::Black), 0);
        // f6: svarta bönderna på e7 och g7 samt springaren på g8.
        assert_eq!(attackers(&b, Square::F6, Color::Black), 3);
        assert_eq!(attackers(&b, Square::E4, Color::White), 0);
    }

    #[test]
    fn pawns_attack_forward_diagonally_only() {
        let b = board("4k3/8/8/4p3/8/8/4P3/4K3 w - - 0 1");
        assert_eq!(attackers(&b, Square::D4, Color::Black), 1);
        assert_eq!(attackers(&b, Square::F4, Color::Black), 1);
        assert_eq!(attackers(&b, Square::E4, Color::Black), 0);
        assert_eq!(attackers(&b, Square::D6, Color::Black), 0);
        assert_eq!(attackers(&b, Square::D3, Color::White), 1);
        assert_eq!(attackers(&b, Square::D1, Color::White), 1);
    }

    #[test]
    fn sliders_are_blocked_by_pieces() {
        // Tornet på a1 når a7 men inte a8 bakom bonden på a7. Löparen på c1 når h6.
        let b = board("4k3/p7/8/8/8/8/8/R1B1K3 w - - 0 1");
        assert_eq!(attackers(&b, Square::A7, Color::White), 1);
        assert_eq!(attackers(&b, Square::A8, Color::White), 0);
        assert_eq!(attackers(&b, Square::H6, Color::White), 1);
        assert_eq!(attackers(&b, Square::B1, Color::White), 1);
    }

    #[test]
    fn queen_attacks_both_ways_and_king_counts() {
        let b = board("4k3/8/8/8/3Q4/8/8/4K3 w - - 0 1");
        assert_eq!(attackers(&b, Square::D8, Color::White), 1);
        assert_eq!(attackers(&b, Square::H8, Color::White), 1);
        assert_eq!(attackers(&b, Square::E8, Color::White), 0);
        // d2 nås av damen och kungen på e1.
        assert_eq!(attackers(&b, Square::D2, Color::White), 2);
        assert_eq!(attackers(&b, Square::D7, Color::Black), 1);
    }

    #[test]
    fn battery_counts_front_piece_only() {
        // Två torn på d-linjen: bara det främre når d8.
        let b = board("4k3/8/8/8/8/8/3R4/3RK3 w - - 0 1");
        assert_eq!(attackers(&b, Square::D8, Color::White), 1);
    }
}
