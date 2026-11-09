//! HTML för brädet, statusraden och ratingkurvan.
//!
//! Allt som byggs här kommer från våra egna data, aldrig från fri text som spelaren skrivit.
//! Spelarens inmatning visas av webbcratet via `textContent`.

use std::cmp::Ordering;
use std::fmt::Write as _;

use chess_trainer_domain::notation::to_swedish;
use chess_trainer_engine::attacks::{attackers, is_threatened};
use chess_trainer_engine::describe::color_name;
use chess_trainer_engine::san::legal_moves;
use chess_trainer_engine::{Board, Color, File, GameStatus, Piece, Rank, Square};

use crate::commentary::yours;
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

/// Det som ska markeras på brädet utöver pjäserna.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Marks {
    /// Rutan spelaren har valt att flytta från.
    pub selected: Option<Square>,
    /// Färga rutorna efter hur attackerade de är för den här spelaren.
    pub heat_for: Option<Color>,
    /// Senaste dragets från- och till-ruta.
    pub last_move: Option<(Square, Square)>,
    /// Låt pjäsen i senaste draget glida från sin gamla ruta.
    pub animate: bool,
    /// Tipset: pjäsen som bör flyttas och rutan den bör till.
    pub hint: Option<(Square, Square)>,
}

/// Rutans position på skärmen som (kolumn, rad), räknat från övre vänstra hörnet.
fn screen(square: Square, flipped: bool) -> (i32, i32) {
    let index = |found: Option<usize>| i32::try_from(found.unwrap_or(0)).unwrap_or(0);
    let file = index(File::ALL.iter().position(|f| *f == square.file()));
    let rank = index(Rank::ALL.iter().position(|r| *r == square.rank()));
    if flipped {
        (7_i32.saturating_sub(file), rank)
    } else {
        (file, 7_i32.saturating_sub(rank))
    }
}

/// Hur många rutor pjäsen ska glida, från `from` till `to`, i skärmens riktning.
fn slide(from: Square, to: Square, flipped: bool) -> (i32, i32) {
    let (from_col, from_row) = screen(from, flipped);
    let (to_col, to_row) = screen(to, flipped);
    (
        from_col.saturating_sub(to_col),
        from_row.saturating_sub(to_row),
    )
}

/// Det som läggs till en ruta utöver klasserna.
struct Extra {
    /// Läggs till skärmläsartexten.
    label_suffix: String,
    /// Hur många rutor pjäsen ska glida in från, om den ska animeras.
    motion: Option<(i32, i32)>,
}

/// En ruta som knapp.
fn square_html(board: &Board, square: Square, classes: &str, extra: &Extra) -> String {
    let (content, label) = match (board.piece_on(square), board.color_on(square)) {
        (Some(piece), Some(color)) => {
            let side = if color == Color::White { "w" } else { "b" };
            let name = format!(
                "{square} {} {}",
                color_name(color).to_lowercase(),
                piece_name(piece)
            );
            let slide_attrs = extra.motion.map_or_else(String::new, |(dx, dy)| {
                format!(" slide\" style=\"--dx:{dx};--dy:{dy}")
            });
            (
                format!(
                    "<span class=\"pc {side}{slide_attrs}\">{}{TEXT_PRESENTATION}</span>",
                    glyph(piece)
                ),
                name,
            )
        }
        _ => (String::new(), square.to_string()),
    };
    format!(
        "<button class=\"{classes}\" data-sq=\"{square}\" aria-label=\"{label}{}\">{content}</button>",
        extra.label_suffix
    )
}

/// Klass och skärmläsartext för schack och matt på en ruta.
fn check_marks(board: &Board, square: Square) -> (&'static str, &'static str) {
    let checkers = board.checkers();
    if checkers.is_empty() {
        return ("", "");
    }
    let mated = board.status() == GameStatus::Won;
    if square == board.king(board.side_to_move()) {
        if mated {
            (" mated", ", schack matt")
        } else {
            (" in-check", ", i schack")
        }
    } else if mated && checkers.has(square) {
        (" mating", ", ger matt")
    } else {
        ("", "")
    }
}

/// Hela brädet. Vänt med svart nederst när `flipped` är sant.
///
/// Med `marks.heat_for` färgas rutorna efter hur attackerade de är för den spelaren:
/// röd där motståndaren har fler attacker, grön där spelaren har fler, gul vid lika.
#[must_use]
pub fn board_html(board: &Board, flipped: bool, marks: Marks) -> String {
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
            if marks.selected == Some(square) {
                classes.push_str(" sel");
            }
            if marks
                .last_move
                .is_some_and(|(from, to)| square == from || square == to)
            {
                classes.push_str(" last");
            }
            let mut label_suffix = marks.heat_for.map_or_else(String::new, |player| {
                let (class, label) = heat(
                    attackers(board, square, player),
                    attackers(board, square, !player),
                );
                classes.push_str(&class);
                label
            });
            let (check_class, check_label) = check_marks(board, square);
            classes.push_str(check_class);
            label_suffix.push_str(check_label);
            if let Some((from, to)) = marks.hint {
                if square == from {
                    classes.push_str(" hint-from");
                    label_suffix.push_str(", tips: flytta den här pjäsen");
                }
                if square == to {
                    classes.push_str(" hint-to");
                    label_suffix.push_str(", tips: hit");
                }
            }
            let motion = marks
                .last_move
                .filter(|(_, to)| marks.animate && *to == square)
                .map(|(from, to)| slide(from, to, flipped));
            let extra = Extra {
                label_suffix,
                motion,
            };
            out.push_str(&square_html(board, square, &classes, &extra));
        }
    }
    out
}

/// Högst så många hotade pjäser varnas det för åt gången.
const MAX_WARNINGS: usize = 3;

/// Rutan över brädet när partiet är slut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Banner {
    /// Stor rubrik, som "Schack matt!".
    pub title: &'static str,
    /// En mening under rubriken.
    pub text: &'static str,
}

/// Resultatrutan, när partiet är slut.
#[must_use]
pub fn banner(game: &Game) -> Option<Banner> {
    let (title, text) = match game.score()? {
        Score::Win => ("Schack matt!", "Du vann. Snyggt spelat!"),
        Score::Loss if game.resigned() => ("Du gav upp", "Kolla lärdomarna och försök igen."),
        Score::Loss => (
            "Schack matt",
            "Motorn vann den här gången. Kolla lärdomarna och försök igen.",
        ),
        Score::Draw if legal_moves(game.board()).is_empty() => (
            "Patt",
            "Den som är vid draget kan inte flytta men står inte i schack. Det blir remi.",
        ),
        Score::Draw => ("Remi", "Partiet slutade oavgjort."),
    };
    Some(Banner { title, text })
}

/// Varningar när det är spelarens tur: schack och pjäser som kan slås.
#[must_use]
pub fn warnings(game: &Game) -> Vec<String> {
    if game.is_over() || game.engine_to_move() {
        return Vec::new();
    }
    let board = game.board();
    let player = game.player();
    let mut out = Vec::new();
    if !board.checkers().is_empty() {
        out.push("Du står i schack! Skydda kungen.".to_owned());
    }
    out.extend(
        board
            .colors(player)
            .into_iter()
            .filter(|square| is_threatened(board, *square, !player))
            .take(MAX_WARNINGS)
            .map(|square| {
                let piece = board.piece_on(square).unwrap_or(Piece::Pawn);
                let threatened = if piece == Piece::Rook {
                    "hotat"
                } else {
                    "hotad"
                };
                format!("Se upp: {} på {square} är {threatened}!", yours(piece))
            }),
    );
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
    use super::{
        Board, Color, Marks, Square, banner, board_html, heat, move_list, rating_svg, slide,
        status_text, warnings,
    };
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
        let html = board_html(&Board::default(), false, Marks::default());
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
        let html = board_html(&Board::default(), false, Marks::default());
        assert!(html.contains("class=\"sq dark\" data-sq=\"a1\""));
        assert!(html.contains("class=\"sq light\" data-sq=\"b1\""));
        assert!(html.contains("class=\"sq light\" data-sq=\"h1\""));
        assert!(html.contains("class=\"sq dark\" data-sq=\"b2\""));
    }

    #[test]
    fn flipped_board_starts_at_h1() {
        let html = board_html(&Board::default(), true, Marks::default());
        assert!(html.starts_with("<button class=\"sq light\" data-sq=\"h1\""));
        assert!(html.contains("class=\"sq dark\" data-sq=\"a1\""));
        assert!(html.ends_with(
            "data-sq=\"a8\" aria-label=\"a8 svart torn\"><span class=\"pc b\">♜\u{FE0E}</span></button>"
        ));
    }

    #[test]
    fn every_piece_type_is_drawn() {
        let html = board_html(&Board::default(), false, Marks::default());
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
        let html = board_html(
            &Board::default(),
            false,
            Marks {
                selected: Some(Square::E2),
                ..Marks::default()
            },
        );
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
            Marks {
                selected: Some(Square::E2),
                heat_for: Some(Color::White),
                ..Marks::default()
            },
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
        let html = board_html(
            &Board::default(),
            true,
            Marks {
                heat_for: Some(Color::Black),
                ..Marks::default()
            },
        );
        assert!(html.contains("class=\"sq dark heat-safe-3\" data-sq=\"f6\""));
        assert!(html.contains("class=\"sq light heat-danger-3\" data-sq=\"f3\""));
    }

    #[test]
    fn last_move_is_marked_and_slides_when_animated() {
        let board: Board = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
            .parse()
            .unwrap();
        let last = Marks {
            last_move: Some((Square::E2, Square::E4)),
            ..Marks::default()
        };
        let html = board_html(&board, false, last);
        assert!(html.contains("class=\"sq light last\" data-sq=\"e2\""));
        assert!(html.contains("class=\"sq light last\" data-sq=\"e4\""));
        assert!(!html.contains("slide"));
        let animated = board_html(
            &board,
            false,
            Marks {
                animate: true,
                ..last
            },
        );
        assert!(animated.contains("<span class=\"pc w slide\" style=\"--dx:0;--dy:2\">♟"));
        assert_eq!(animated.matches("slide").count(), 1);
        let flipped = board_html(
            &board,
            true,
            Marks {
                animate: true,
                ..last
            },
        );
        assert!(flipped.contains("style=\"--dx:0;--dy:-2\""));
    }

    #[test]
    fn hint_marks_the_piece_and_the_target() {
        let marks = Marks {
            hint: Some((Square::G1, Square::F3)),
            ..Marks::default()
        };
        let html = board_html(&Board::default(), false, marks);
        assert!(html.contains(
            "class=\"sq dark hint-from\" data-sq=\"g1\" aria-label=\"g1 vit springare, tips: flytta den här pjäsen\""
        ));
        assert!(
            html.contains("class=\"sq light hint-to\" data-sq=\"f3\" aria-label=\"f3, tips: hit\"")
        );
        assert_eq!(html.matches("hint-").count(), 2);
    }

    #[test]
    fn slide_direction_follows_the_screen() {
        // Springare g1-f3: en kolumn åt höger och två rader ner, sett från f3.
        assert_eq!(slide(Square::G1, Square::F3, false), (1, 2));
        assert_eq!(slide(Square::G1, Square::F3, true), (-1, -2));
        assert_eq!(slide(Square::A8, Square::H1, false), (-7, -7));
        assert_eq!(slide(Square::E4, Square::E4, false), (0, 0));
    }

    #[test]
    fn check_and_mate_are_marked_on_the_board() {
        let check: Board = "4k3/8/8/8/8/8/8/4K2r w - - 0 1".parse().unwrap();
        let html = board_html(&check, false, Marks::default());
        assert!(html.contains(
            "class=\"sq dark in-check\" data-sq=\"e1\" aria-label=\"e1 vit kung, i schack\""
        ));
        assert!(!html.contains("mating"));
        let mate: Board = "R5k1/5ppp/8/8/8/8/8/4K3 b - - 0 1".parse().unwrap();
        let mated = board_html(&mate, false, Marks::default());
        assert!(mated.contains(
            "class=\"sq light mated\" data-sq=\"g8\" aria-label=\"g8 svart kung, schack matt\""
        ));
        assert!(mated.contains(
            "class=\"sq light mating\" data-sq=\"a8\" aria-label=\"a8 vit torn, ger matt\""
        ));
        assert_eq!(mated.matches("mating").count(), 1);
        let quiet = board_html(&Board::default(), false, Marks::default());
        assert!(!quiet.contains("check") && !quiet.contains("mate"));
    }

    #[test]
    fn banner_for_every_ending() {
        assert_eq!(banner(&Game::new(Color::White, 1000.0, 1)), None);
        let mut win = Game::from_fen("6k1/5ppp/8/8/8/8/8/R3K3 w - - 0 1", Color::White);
        win.play("Ta8").unwrap();
        assert_eq!(banner(&win).unwrap().title, "Schack matt!");
        let lost = Game::from_fen("R5k1/5ppp/8/8/8/8/8/4K3 b - - 0 1", Color::Black);
        assert_eq!(banner(&lost).unwrap().title, "Schack matt");
        assert!(banner(&lost).unwrap().text.starts_with("Motorn vann"));
        let mut resigned = Game::new(Color::White, 1000.0, 1);
        resigned.resign();
        assert_eq!(banner(&resigned).unwrap().title, "Du gav upp");
        let stalemate = Game::from_fen("7k/8/6Q1/8/8/8/8/K7 b - - 0 1", Color::White);
        assert_eq!(banner(&stalemate).unwrap().title, "Patt");
        let fifty = Game::from_fen("4k3/8/8/8/8/8/8/4K2R w - - 100 80", Color::White);
        assert_eq!(banner(&fifty).unwrap().title, "Remi");
    }

    #[test]
    fn warnings_for_check_and_threatened_pieces() {
        let quiet = Game::new(Color::White, 1000.0, 1);
        assert!(warnings(&quiet).is_empty());
        let check = Game::from_fen("4k3/8/8/8/8/8/8/4K2r w - - 0 1", Color::White);
        assert_eq!(warnings(&check), vec!["Du står i schack! Skydda kungen."]);
        let knight = Game::from_fen("4k3/8/8/8/4p3/5N2/8/4K3 w - - 0 1", Color::White);
        assert_eq!(
            warnings(&knight),
            vec!["Se upp: din springare på f3 är hotad!"]
        );
        let rook = Game::from_fen("4k3/8/8/8/8/8/1b6/R3K3 w - - 0 1", Color::White);
        assert_eq!(warnings(&rook), vec!["Se upp: ditt torn på a1 är hotat!"]);
        let many = Game::from_fen("4k3/8/8/8/1p1p1p1p/N1N1N1N1/8/4K3 w - - 0 1", Color::White);
        assert_eq!(warnings(&many).len(), 3);
    }

    #[test]
    fn no_warnings_on_the_engines_turn_or_after_the_game() {
        let mut game = Game::from_fen("4k3/8/8/8/4p3/5N2/8/4K3 w - - 0 1", Color::White);
        game.play("Kd1").unwrap();
        assert!(warnings(&game).is_empty());
        let mut over = Game::from_fen("4k3/8/8/8/4p3/5N2/8/4K3 w - - 0 1", Color::White);
        over.resign();
        assert!(warnings(&over).is_empty());
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
