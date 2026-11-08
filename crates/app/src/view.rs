//! HTML för brädet, statusraden och ratingkurvan.
//!
//! Allt som byggs här kommer från våra egna data, aldrig från fri text som spelaren skrivit.
//! Spelarens inmatning visas av webbcratet via `textContent`.

use std::cmp::Ordering;
use std::fmt::Write as _;

use chess_trainer_domain::notation::to_swedish;
use chess_trainer_engine::attacks::attackers;
use chess_trainer_engine::describe::color_name;
use chess_trainer_engine::{Board, Color, File, Piece, Rank, Square};

use crate::game::Game;
use crate::knowledge::{KnowledgeBase, Score};

/// Variationsväljare som ber om textform. Utan den ritar iOS bonden ♟ som en svart emoji,
/// och då går färgen inte att ändra med CSS.
const TEXT_PRESENTATION: char = '\u{FE0E}';

/// Unicode-tecken för en pjäs. Samma fyllda tecken för båda färgerna; färgen sätts med CSS.
const fn glyph(piece: Piece) -> char {
    match piece {
        Piece::Pawn => '♟',
        Piece::Knight => '♞',
        Piece::Bishop => '♝',
        Piece::Rook => '♜',
        Piece::Queen => '♛',
        Piece::King => '♚',
    }
}

/// Svenskt namn på en pjäs, för skärmläsare.
const fn piece_name(piece: Piece) -> &'static str {
    match piece {
        Piece::Pawn => "bonde",
        Piece::Knight => "springare",
        Piece::Bishop => "löpare",
        Piece::Rook => "torn",
        Piece::Queen => "dam",
        Piece::King => "kung",
    }
}

/// Färgklass och skärmläsartext för hur attackerad en ruta är, ur spelarens perspektiv.
/// `ours` och `theirs` är antalet attacker från spelaren och från motståndaren.
fn heat(ours: u32, theirs: u32) -> (String, String) {
    if ours == 0 && theirs == 0 {
        return (String::new(), String::new());
    }
    let class = match ours.cmp(&theirs) {
        Ordering::Less => format!(" heat-danger-{}", theirs.saturating_sub(ours).min(3)),
        Ordering::Greater => format!(" heat-safe-{}", ours.saturating_sub(theirs).min(3)),
        Ordering::Equal => " heat-even".to_owned(),
    };
    (class, format!(", du {ours}, motståndaren {theirs}"))
}

/// En ruta som knapp. `label_suffix` läggs till skärmläsartexten.
fn square_html(board: &Board, square: Square, classes: &str, label_suffix: &str) -> String {
    let (content, label) = match (board.piece_on(square), board.color_on(square)) {
        (Some(piece), Some(color)) => {
            let side = if color == Color::White { "w" } else { "b" };
            let name = format!(
                "{square} {} {}",
                color_name(color).to_lowercase(),
                piece_name(piece)
            );
            (
                format!(
                    "<span class=\"pc {side}\">{}{TEXT_PRESENTATION}</span>",
                    glyph(piece)
                ),
                name,
            )
        }
        _ => (String::new(), square.to_string()),
    };
    format!(
        "<button class=\"{classes}\" data-sq=\"{square}\" aria-label=\"{label}{label_suffix}\">{content}</button>"
    )
}

/// Hela brädet. Vänt med svart nederst när `flipped` är sant.
///
/// Med `heat_for` färgas rutorna efter hur attackerade de är för den spelaren:
/// röd där motståndaren har fler attacker, grön där spelaren har fler, gul vid lika.
#[must_use]
pub fn board_html(
    board: &Board,
    flipped: bool,
    selected: Option<Square>,
    heat_for: Option<Color>,
) -> String {
    let mut ranks: Vec<(usize, Rank)> = Rank::ALL.into_iter().enumerate().collect();
    let mut files: Vec<(usize, File)> = File::ALL.into_iter().enumerate().collect();
    if flipped {
        files.reverse();
    } else {
        ranks.reverse();
    }
    let mut out = String::new();
    for (rank_index, rank) in &ranks {
        for (file_index, file) in &files {
            let square = Square::new(*file, *rank);
            let light = (rank_index ^ file_index) & 1 == 1;
            let mut classes = String::from(if light { "sq light" } else { "sq dark" });
            if selected == Some(square) {
                classes.push_str(" sel");
            }
            let suffix = heat_for.map_or_else(String::new, |player| {
                let (class, label) = heat(
                    attackers(board, square, player),
                    attackers(board, square, !player),
                );
                classes.push_str(&class);
                label
            });
            out.push_str(&square_html(board, square, &classes, &suffix));
        }
    }
    out
}

/// Statusraden: vem som spelar vad, och resultatet när partiet är slut.
#[must_use]
pub fn status_text(game: &Game) -> String {
    let level = game.level().rating;
    let color = color_name(game.player()).to_lowercase();
    match game.score() {
        None => format!("Du spelar {color} mot motorn på nivå {level}. Ditt drag."),
        Some(Score::Win) => "Du vann! Snyggt.".to_owned(),
        Some(Score::Draw) => "Remi.".to_owned(),
        Some(Score::Loss) => "Du förlorade. Kolla lärdomarna nedan.".to_owned(),
    }
}

/// Dragen numrerade, som "1. e4 e5 2. Sf3".
#[must_use]
pub fn move_list(moves: &[String]) -> String {
    let mut out = String::new();
    for (number, pair) in (1_u32..).zip(moves.chunks(2)) {
        if !out.is_empty() {
            out.push(' ');
        }
        let _ = write!(out, "{number}. {}", to_swedish(&pair.join(" ")));
    }
    out
}

/// Ratingkurvan som inline-SVG. Tom sträng innan första partiet.
#[must_use]
pub fn rating_svg(kb: &KnowledgeBase) -> String {
    if kb.games.is_empty() {
        return String::new();
    }
    let ratings: Vec<f64> = std::iter::once(crate::knowledge::START_RATING)
        .chain(kb.games.iter().map(|g| g.rating_after))
        .collect();
    let low = ratings.iter().copied().fold(f64::INFINITY, f64::min) - 20.0;
    let high = ratings.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 20.0;
    let last = f64::from(u32::try_from(ratings.len()).unwrap_or(u32::MAX)) - 1.0;
    let points: Vec<String> = (0_u32..)
        .zip(&ratings)
        .map(|(i, rating)| {
            let x = 300.0 * f64::from(i) / last;
            let y = 80.0 - 80.0 * (rating - low) / (high - low);
            format!("{x:.1},{y:.1}")
        })
        .collect();
    format!(
        "<svg viewBox=\"0 0 300 80\" role=\"img\" aria-label=\"Uppskattad rating över tid, nu {:.0}\">\
         <polyline fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" points=\"{}\"/></svg>",
        kb.rating,
        points.join(" ")
    )
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    reason = "testnamnen är dokumentationen"
)]
mod tests {
    use super::{Board, Color, Square, board_html, heat, move_list, rating_svg, status_text};
    use crate::game::Game;
    use crate::knowledge::{GameRecord, KnowledgeBase, Score};

    fn record(score: Score) -> GameRecord {
        GameRecord {
            played_at: String::new(),
            player_white: true,
            moves: vec![],
            score,
            opponent_rating: 1000.0,
            rating_after: 0.0,
            mistakes: vec![],
        }
    }

    #[test]
    fn board_has_sixty_four_squares_from_a8() {
        let html = board_html(&Board::default(), false, None, None);
        assert_eq!(html.matches("<button").count(), 64);
        assert!(html.starts_with(
            "<button class=\"sq light\" data-sq=\"a8\" aria-label=\"a8 svart torn\">"
        ));
        assert!(html.ends_with(
            "data-sq=\"h1\" aria-label=\"h1 vit torn\"><span class=\"pc w\">♜\u{FE0E}</span></button>"
        ));
        assert!(html.contains("aria-label=\"e4\"></button>"));
    }

    #[test]
    fn colours_alternate_and_a1_is_dark() {
        let html = board_html(&Board::default(), false, None, None);
        assert!(html.contains("class=\"sq dark\" data-sq=\"a1\""));
        assert!(html.contains("class=\"sq light\" data-sq=\"b1\""));
        assert!(html.contains("class=\"sq light\" data-sq=\"h1\""));
        assert!(html.contains("class=\"sq dark\" data-sq=\"b2\""));
    }

    #[test]
    fn flipped_board_starts_at_h1() {
        let html = board_html(&Board::default(), true, None, None);
        assert!(html.starts_with("<button class=\"sq light\" data-sq=\"h1\""));
        assert!(html.contains("class=\"sq dark\" data-sq=\"a1\""));
        assert!(html.ends_with(
            "data-sq=\"a8\" aria-label=\"a8 svart torn\"><span class=\"pc b\">♜\u{FE0E}</span></button>"
        ));
    }

    #[test]
    fn every_piece_type_is_drawn() {
        let html = board_html(&Board::default(), false, None, None);
        for (glyph, name) in [
            ('♟', "bonde"),
            ('♞', "springare"),
            ('♝', "löpare"),
            ('♛', "dam"),
            ('♚', "kung"),
        ] {
            assert!(html.contains(&format!("{glyph}\u{FE0E}</span>")));
            assert!(html.contains(name));
        }
    }

    #[test]
    fn selected_square_is_marked() {
        let html = board_html(&Board::default(), false, Some(Square::E2), None);
        assert!(html.contains("class=\"sq light sel\" data-sq=\"e2\""));
        assert_eq!(html.matches(" sel").count(), 1);
    }

    #[test]
    fn heat_classes_and_labels() {
        assert_eq!(heat(0, 0), (String::new(), String::new()));
        assert_eq!(heat(1, 1).0, " heat-even");
        assert_eq!(heat(2, 1).0, " heat-safe-1");
        assert_eq!(heat(5, 0).0, " heat-safe-3");
        assert_eq!(heat(1, 3).0, " heat-danger-2");
        assert_eq!(heat(0, 5).0, " heat-danger-3");
        assert_eq!(heat(1, 2).1, ", du 1, motståndaren 2");
    }

    #[test]
    fn heat_marks_squares_for_the_player() {
        let html = board_html(
            &Board::default(),
            false,
            Some(Square::E2),
            Some(Color::White),
        );
        // f3: bönderna på e2 och g2 samt springaren på g1.
        assert!(html.contains(
            "class=\"sq light heat-safe-3\" data-sq=\"f3\" aria-label=\"f3, du 3, motståndaren 0\""
        ));
        // d3: bönderna på c2 och e2.
        assert!(html.contains("class=\"sq light heat-safe-2\" data-sq=\"d3\""));
        // e2 är vald och skyddas av kung, dam, löpare och springare.
        assert!(html.contains("class=\"sq light sel heat-safe-3\" data-sq=\"e2\""));
        // f6 är svarts ruta.
        assert!(html.contains("class=\"sq dark heat-danger-3\" data-sq=\"f6\""));
        // e4 attackeras inte av någon.
        assert!(html.contains("class=\"sq light\" data-sq=\"e4\" aria-label=\"e4\""));
    }

    #[test]
    fn heat_follows_the_players_colour() {
        let html = board_html(&Board::default(), true, None, Some(Color::Black));
        assert!(html.contains("class=\"sq dark heat-safe-3\" data-sq=\"f6\""));
        assert!(html.contains("class=\"sq light heat-danger-3\" data-sq=\"f3\""));
    }

    #[test]
    fn status_for_ongoing_and_finished_games() {
        let mut game = Game::new(Color::White, 1000.0, 1);
        assert_eq!(
            status_text(&game),
            "Du spelar vit mot motorn på nivå 1000. Ditt drag."
        );
        game.resign();
        assert_eq!(status_text(&game), "Du förlorade. Kolla lärdomarna nedan.");
    }

    #[test]
    fn status_for_win_and_draw() {
        let mut win = Game::from_fen("6k1/5ppp/8/8/8/8/8/R3K3 w - - 0 1", Color::White);
        win.play("Ta8").unwrap();
        assert_eq!(status_text(&win), "Du vann! Snyggt.");
        let draw = Game::from_fen("7k/8/6Q1/8/8/8/8/K7 b - - 0 1", Color::White);
        assert_eq!(status_text(&draw), "Remi.");
        let black = Game::new(Color::Black, 1000.0, 1);
        assert!(status_text(&black).starts_with("Du spelar svart"));
    }

    #[test]
    fn move_list_numbers_pairs() {
        let moves: Vec<String> = ["e4", "e5", "Nf3"]
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(move_list(&moves), "1. e4 e5 2. Sf3");
        assert_eq!(move_list(&[]), "");
    }

    #[test]
    fn rating_svg_is_empty_before_first_game() {
        assert_eq!(rating_svg(&KnowledgeBase::default()), "");
    }

    #[test]
    fn rating_svg_draws_one_point_per_game_plus_start() {
        let mut kb = KnowledgeBase::default();
        kb.add(record(Score::Win));
        kb.add(record(Score::Loss));
        let svg = rating_svg(&kb);
        assert!(svg.contains("nu 999"), "{svg}");
        // Start 1000, sedan 1016, sedan 999. Skala 979..1036.
        assert!(
            svg.contains("points=\"0.0,50.5 150.0,28.1 300.0,51.9\""),
            "{svg}"
        );
    }
}
