//! Vilka pjäser som attackerar en ruta: för färgningen av brädet och för motorns kommentarer.

use cozy_chess::{
    BitBoard, Board, Color, Piece, Square, get_bishop_moves, get_king_moves, get_knight_moves,
    get_pawn_attacks, get_rook_moves,
};

use crate::search::value;

/// Antal pjäser av slaget `piece` och färgen `color` som attackerar `square`.
fn reach(board: &Board, square: Square, color: Color, piece: Piece) -> u32 {
    let occupied = board.occupied();
    let count = |squares: BitBoard| (squares & board.colored_pieces(color, piece)).len();
    match piece {
        // En bonde av `color` attackerar rutan om den står där en motståndarbonde på rutan skulle slå.
        Piece::Pawn => count(get_pawn_attacks(square, !color)),
        Piece::Knight => count(get_knight_moves(square)),
        Piece::Bishop => count(get_bishop_moves(square, occupied)),
        Piece::Rook => count(get_rook_moves(square, occupied)),
        Piece::Queen => count(get_bishop_moves(square, occupied))
            .saturating_add(count(get_rook_moves(square, occupied))),
        Piece::King => count(get_king_moves(square)),
    }
}

/// Antal pjäser av färgen `color` som attackerar `square`.
///
/// Räknar varje pjäs som når rutan direkt. Bundna pjäser räknas, och pjäser bakom
/// varandra på samma linje (batterier) räknas bara en gång.
// ponytail: inga röntgenattacker; räkna batterier om färgningen visar sig missvisande.
#[must_use]
pub fn attackers(board: &Board, square: Square, color: Color) -> u32 {
    Piece::ALL
        .iter()
        .map(|piece| reach(board, square, color, *piece))
        .sum()
}

/// Den billigaste pjäsen av färgen `color` som attackerar `square`.
#[must_use]
pub fn least_attacker(board: &Board, square: Square, color: Color) -> Option<Piece> {
    // Piece::ALL är ordnad från bonde till kung, alltså billigast först.
    Piece::ALL
        .into_iter()
        .find(|piece| reach(board, square, color, *piece) > 0)
}

/// Står pjäsen på `square` i fara från `attacker`? Det gör den om den kan slås och antingen
/// saknar skydd eller kan slås av en billigare pjäs. Kungen räknas aldrig här, det är schack.
#[must_use]
pub fn is_threatened(board: &Board, square: Square, attacker: Color) -> bool {
    let Some(piece) = board.piece_on(square) else {
        return false;
    };
    if piece == Piece::King || board.color_on(square) == Some(attacker) {
        return false;
    }
    least_attacker(board, square, attacker).is_some_and(|cheapest| {
        value(cheapest) < value(piece) || attackers(board, square, !attacker) == 0
    })
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    clippy::shadow_unrelated,
    reason = "testnamnen är dokumentationen och fen återanvänds per testfall"
)]
mod tests {
    use super::{Board, Color, Piece, Square, attackers, is_threatened, least_attacker};

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

    #[test]
    fn least_attacker_is_the_cheapest() {
        // d5 attackeras av bonden på e4, springaren på f4 och damen på d1.
        let b = board("4k3/8/8/8/4PN2/8/8/3QK3 w - - 0 1");
        assert_eq!(
            least_attacker(&b, Square::D5, Color::White),
            Some(Piece::Pawn)
        );
        // e6 attackeras bara av springaren på f4.
        assert_eq!(
            least_attacker(&b, Square::E6, Color::White),
            Some(Piece::Knight)
        );
        // a4 attackeras av damen.
        assert_eq!(
            least_attacker(&b, Square::A4, Color::White),
            Some(Piece::Queen)
        );
        assert_eq!(least_attacker(&b, Square::H8, Color::White), None);
        // Löpare, torn och kung.
        let b = board("4k3/8/8/8/8/8/8/R1B1K3 w - - 0 1");
        assert_eq!(
            least_attacker(&b, Square::D2, Color::White),
            Some(Piece::Bishop)
        );
        assert_eq!(
            least_attacker(&b, Square::A5, Color::White),
            Some(Piece::Rook)
        );
        assert_eq!(
            least_attacker(&b, Square::F2, Color::White),
            Some(Piece::King)
        );
    }

    #[test]
    fn threats() {
        // Svart springare på d5: oskyddad och attackerad av damen på d1.
        let b = board("4k3/8/8/3n4/8/8/8/3QK3 w - - 0 1");
        assert!(is_threatened(&b, Square::D5, Color::White));
        // Skyddad av bonden på e6 och bara attackerad av damen: ingen fara.
        let b = board("4k3/8/4p3/3n4/8/8/8/3QK3 w - - 0 1");
        assert!(!is_threatened(&b, Square::D5, Color::White));
        // Skyddad men attackerad av en billigare bonde: fara.
        let b = board("4k3/8/4p3/3n4/4P3/8/8/3QK3 w - - 0 1");
        assert!(is_threatened(&b, Square::D5, Color::White));
        // Tom ruta, egen pjäs och kung räknas inte.
        assert!(!is_threatened(&b, Square::A3, Color::White));
        assert!(!is_threatened(&b, Square::E4, Color::White));
        let check = board("4k3/8/8/8/8/8/8/4RK2 b - - 0 1");
        assert!(!is_threatened(&check, Square::E8, Color::White));
        // Oattackerad pjäs är inte hotad.
        assert!(!is_threatened(&b, Square::E6, Color::White));
    }

    #[test]
    fn equal_value_attacker_on_defended_piece_is_no_threat() {
        // Svart springare på d5 skyddad av bonden på e6, attackerad av vit springare på f4.
        let b = board("4k3/8/4p3/3n4/5N2/8/8/4K3 w - - 0 1");
        assert!(!is_threatened(&b, Square::D5, Color::White));
    }
}
