//! Beskriver ställningen i ord, för blindspel.

use cozy_chess::{Board, Color, Piece, Square};

/// Svensk pjäsbokstav. Bönder skrivs utan bokstav.
const fn swedish_letter(piece: Piece) -> &'static str {
    match piece {
        Piece::Pawn => "",
        Piece::Knight => "S",
        Piece::Bishop => "L",
        Piece::Rook => "T",
        Piece::Queen => "D",
        Piece::King => "K",
    }
}

/// Svenskt namn på färgen.
#[must_use]
pub const fn color_name(color: Color) -> &'static str {
    match color {
        Color::White => "Vit",
        Color::Black => "Svart",
    }
}

/// En färgs pjäser, största först: "Kg1, Dd1, Ta1, e4".
fn pieces_of(board: &Board, color: Color) -> String {
    let mut parts = Vec::new();
    for piece in Piece::ALL.iter().rev() {
        let squares: Vec<Square> = (board.pieces(*piece) & board.colors(color))
            .iter()
            .collect();
        parts.extend(
            squares
                .iter()
                .map(|sq| format!("{}{sq}", swedish_letter(*piece))),
        );
    }
    parts.join(", ")
}

/// Hela ställningen i ord, med vem som är vid draget och om det är schack.
#[must_use]
pub fn describe(board: &Board) -> String {
    let to_move = color_name(board.side_to_move());
    let check = if board.checkers().is_empty() {
        String::new()
    } else {
        format!(" {to_move} står i schack.")
    };
    format!(
        "Vit: {}. Svart: {}. {to_move} vid draget.{check}",
        pieces_of(board, Color::White),
        pieces_of(board, Color::Black),
    )
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    clippy::shadow_unrelated,
    reason = "testnamnen är dokumentationen och fen återanvänds per testfall"
)]
mod tests {
    use super::{Board, Color, color_name, describe};

    #[test]
    fn color_names() {
        assert_eq!(color_name(Color::White), "Vit");
        assert_eq!(color_name(Color::Black), "Svart");
    }

    #[test]
    fn describes_small_position() {
        let board: Board = "4k3/8/8/8/4P3/8/8/R3K1N1 w - - 0 1".parse().unwrap();
        assert_eq!(
            describe(&board),
            "Vit: Ke1, Ta1, Sg1, e4. Svart: Ke8. Vit vid draget."
        );
    }

    #[test]
    fn mentions_check_and_side_to_move() {
        let board: Board = "4k3/8/8/8/8/8/8/3QK2r w - - 0 1".parse().unwrap();
        assert_eq!(
            describe(&board),
            "Vit: Ke1, Dd1. Svart: Ke8, Th1. Vit vid draget. Vit står i schack."
        );
    }

    #[test]
    fn black_to_move_with_bishop() {
        let board: Board = "4k3/8/8/1b6/8/8/8/4K3 b - - 0 1".parse().unwrap();
        assert_eq!(
            describe(&board),
            "Vit: Ke1. Svart: Ke8, Lb5. Svart vid draget."
        );
    }
}
