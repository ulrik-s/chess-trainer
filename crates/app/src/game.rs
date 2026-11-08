//! Ett parti mellan spelaren och motorn.

use std::fmt;

use chess_trainer_domain::notation::to_san;
use chess_trainer_engine::describe::describe;
use chess_trainer_engine::mistakes::{Mistake, assess};
use chess_trainer_engine::san::{from_squares, landing_square, parse_san, san};
use chess_trainer_engine::search::{Rng, choose};
use chess_trainer_engine::{Board, Color, GameStatus, Move, Square};

use crate::commentary::comment;
use crate::knowledge::{GameRecord, MistakeRecord, Score};
use crate::level::{Level, level_for};

/// Varför ett drag inte kunde spelas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    /// Texten gick inte att tolka som ett drag.
    Unrecognised,
    /// Draget är inte lagligt i ställningen.
    Illegal,
    /// Partiet är redan slut.
    GameOver,
    /// Det är motorns tur.
    NotYourTurn,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unrecognised => {
                "Jag känner inte igen draget. Skriv till exempel e4, Sf3 eller springare f3."
            }
            Self::Illegal => "Det draget går inte i den här ställningen.",
            Self::GameOver => "Partiet är slut. Starta ett nytt.",
            Self::NotYourTurn => "Vänta, motorn tänker.",
        })
    }
}

/// Vad som hände när spelaren gjorde ett drag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    /// Spelarens drag i SAN.
    pub played: String,
    /// Felet, om draget var ett tydligt fel.
    pub mistake: Option<Mistake>,
}

/// Motorns drag, med kommentaren om hur den tänkte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineMove {
    /// Draget i SAN.
    pub san: String,
    /// Motorns förklaring i första person.
    pub comment: String,
}

/// Ett pågående eller avslutat parti.
#[derive(Debug, Clone)]
pub struct Game {
    /// Aktuell ställning.
    board: Board,
    /// Alla drag hittills i SAN.
    moves: Vec<String>,
    /// Spelarens färg.
    player: Color,
    /// Motorns nivå, vald från spelarens rating när partiet startade.
    level: Level,
    /// Slumpen bakom motorns val.
    rng: Rng,
    /// Spelarens fel hittills.
    mistakes: Vec<MistakeRecord>,
    /// Har spelaren gett upp?
    resigned: bool,
    /// Har partiet redan sparats i kunskapsbasen?
    recorded: bool,
    /// Senaste dragets från- och till-ruta, så som en människa ser dem.
    last_move: Option<(Square, Square)>,
}

impl Game {
    /// Nytt parti. Spelar spelaren svart börjar motorn, via [`Game::engine_move`].
    #[must_use]
    pub fn new(player: Color, rating: f64, seed: u64) -> Self {
        Self {
            board: Board::default(),
            moves: Vec::new(),
            player,
            level: level_for(rating),
            rng: Rng::new(seed),
            mistakes: Vec::new(),
            resigned: false,
            recorded: false,
            last_move: None,
        }
    }

    /// Parti från en given ställning, för tester.
    #[cfg(test)]
    pub(crate) fn from_fen(fen: &str, player: Color) -> Self {
        let mut game = Self::new(player, 1600.0, 1);
        game.board = fen.parse().unwrap_or_default();
        game.moves.clear();
        game
    }

    /// Aktuell ställning.
    #[must_use]
    pub const fn board(&self) -> &Board {
        &self.board
    }

    /// Alla drag hittills i SAN.
    #[must_use]
    pub fn moves(&self) -> &[String] {
        &self.moves
    }

    /// Spelarens färg.
    #[must_use]
    pub const fn player(&self) -> Color {
        self.player
    }

    /// Motorns nivå.
    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    /// Senaste dragets från- och till-ruta. Vid rockad är det kungens rutor.
    #[must_use]
    pub const fn last_move(&self) -> Option<(Square, Square)> {
        self.last_move
    }

    /// Är det motorns tur att dra?
    #[must_use]
    pub fn engine_to_move(&self) -> bool {
        !self.is_over() && self.board.side_to_move() != self.player
    }

    /// Är partiet slut?
    #[must_use]
    pub fn is_over(&self) -> bool {
        self.resigned || self.board.status() != GameStatus::Ongoing
    }

    /// Spelar ett drag skrivet i svensk eller engelsk notation.
    ///
    /// # Errors
    /// Om texten inte är ett drag, draget är olagligt, partiet är slut eller det är motorns tur.
    pub fn play(&mut self, input: &str) -> Result<Turn, MoveError> {
        self.check_turn()?;
        let wanted = to_san(input).ok_or(MoveError::Unrecognised)?;
        let mv = parse_san(&self.board, &wanted).ok_or(MoveError::Illegal)?;
        Ok(self.apply(mv))
    }

    /// Spelar draget från `from` till `to`, som när spelaren klickar på brädet.
    ///
    /// # Errors
    /// Om draget är olagligt, partiet är slut eller det är motorns tur.
    pub fn play_squares(&mut self, from: Square, to: Square) -> Result<Turn, MoveError> {
        self.check_turn()?;
        let mv = from_squares(&self.board, from, to).ok_or(MoveError::Illegal)?;
        Ok(self.apply(mv))
    }

    /// Får spelaren dra just nu?
    fn check_turn(&self) -> Result<(), MoveError> {
        if self.is_over() {
            Err(MoveError::GameOver)
        } else if self.engine_to_move() {
            Err(MoveError::NotYourTurn)
        } else {
            Ok(())
        }
    }

    /// Bedömer och spelar spelarens drag. Motorn svarar först vid [`Game::engine_move`].
    fn apply(&mut self, mv: Move) -> Turn {
        let mistake = assess(&self.board, mv);
        if let Some(found) = &mistake {
            self.mistakes.push(MistakeRecord {
                move_number: self.board.fullmove_number(),
                played: found.played.clone(),
                best: found.best.clone(),
                kind: found.kind,
            });
        }
        let played = self.push(mv);
        Turn { played, mistake }
    }

    /// Spelar ett drag på brädet och returnerar det i SAN.
    fn push(&mut self, mv: Move) -> String {
        let text = san(&self.board, mv);
        self.last_move = Some((mv.from, landing_square(&self.board, mv)));
        self.board.play_unchecked(mv);
        self.moves.push(text.clone());
        text
    }

    /// Motorns drag och kommentar, om det är motorns tur.
    pub fn engine_move(&mut self) -> Option<EngineMove> {
        if !self.engine_to_move() {
            return None;
        }
        choose(&self.board, self.level.strength, &mut self.rng).map(|choice| {
            let comment = comment(&self.board, &choice);
            let san = self.push(choice.mv);
            EngineMove { san, comment }
        })
    }

    /// Spelaren ger upp.
    pub const fn resign(&mut self) {
        self.resigned = true;
    }

    /// Resultatet ur spelarens perspektiv, när partiet är slut.
    #[must_use]
    pub fn score(&self) -> Option<Score> {
        if self.resigned {
            return Some(Score::Loss);
        }
        match self.board.status() {
            GameStatus::Ongoing => None,
            GameStatus::Drawn => Some(Score::Draw),
            GameStatus::Won => Some(if self.board.side_to_move() == self.player {
                Score::Loss
            } else {
                Score::Win
            }),
        }
    }

    /// Partiet som post för kunskapsbasen. Ges bara en gång, och bara när partiet är slut.
    pub fn take_record(&mut self, played_at: String) -> Option<GameRecord> {
        if self.recorded {
            return None;
        }
        let score = self.score()?;
        self.recorded = true;
        Some(GameRecord {
            played_at,
            player_white: self.player == Color::White,
            moves: self.moves.clone(),
            score,
            opponent_rating: self.level.rating,
            rating_after: 0.0,
            mistakes: self.mistakes.clone(),
        })
    }

    /// Ställningen i ord.
    #[must_use]
    pub fn describe(&self) -> String {
        describe(&self.board)
    }
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    reason = "testnamnen är dokumentationen"
)]
mod tests {
    use super::{Color, Game, MoveError, Score, Square};
    use chess_trainer_engine::mistakes::MistakeKind;

    fn game_from(fen: &str, player: Color) -> Game {
        Game::from_fen(fen, player)
    }

    #[test]
    fn white_starts_with_empty_board_history() {
        let game = Game::new(Color::White, 1000.0, 1);
        assert!(game.moves().is_empty());
        assert_eq!(game.player(), Color::White);
        assert!((game.level().rating - 1000.0).abs() < f64::EPSILON);
        assert!(!game.is_over());
        assert_eq!(game.score(), None);
    }

    #[test]
    fn engine_moves_first_when_player_is_black() {
        let mut game = Game::new(Color::Black, 1000.0, 1);
        assert!(game.moves().is_empty());
        assert!(game.engine_to_move());
        assert_eq!(game.play("e5"), Err(MoveError::NotYourTurn));
        assert_eq!(
            game.play_squares(Square::E7, Square::E5),
            Err(MoveError::NotYourTurn)
        );
        let reply = game.engine_move().unwrap();
        assert!(reply.comment.starts_with("Jag spelar "));
        assert_eq!(game.moves(), [reply.san]);
        assert_eq!(game.board().side_to_move(), Color::Black);
        assert!(!game.engine_to_move());
        assert_eq!(game.engine_move(), None);
    }

    #[test]
    fn swedish_move_gets_an_engine_reply() {
        let mut game = Game::new(Color::White, 1000.0, 1);
        let turn = game.play("springare f3").unwrap();
        assert_eq!(turn.played, "Nf3");
        assert_eq!(turn.mistake, None);
        assert_eq!(game.last_move(), Some((Square::G1, Square::F3)));
        assert!(game.engine_to_move());
        let reply = game.engine_move().unwrap();
        assert!(
            reply
                .comment
                .contains(&chess_trainer_domain::notation::to_swedish(&reply.san))
        );
        assert_eq!(game.moves().len(), 2);
        assert_eq!(game.board().side_to_move(), Color::White);
    }

    #[test]
    fn clicking_squares_plays_a_move() {
        let mut game = Game::new(Color::White, 1000.0, 1);
        let turn = game.play_squares(Square::E2, Square::E4).unwrap();
        assert_eq!(turn.played, "e4");
        assert_eq!(game.moves()[0], "e4");
    }

    #[test]
    fn bad_input_is_rejected_without_changing_the_game() {
        let mut game = Game::new(Color::White, 1000.0, 1);
        assert_eq!(game.play("häst f3"), Err(MoveError::Unrecognised));
        assert_eq!(game.play("e5"), Err(MoveError::Illegal));
        assert_eq!(
            game.play_squares(Square::E2, Square::E5),
            Err(MoveError::Illegal)
        );
        assert!(game.moves().is_empty());
    }

    #[test]
    fn errors_have_swedish_messages() {
        assert!(
            MoveError::Unrecognised
                .to_string()
                .contains("känner inte igen")
        );
        assert!(MoveError::Illegal.to_string().contains("går inte"));
        assert!(MoveError::GameOver.to_string().contains("slut"));
        assert_eq!(MoveError::NotYourTurn.to_string(), "Vänta, motorn tänker.");
    }

    #[test]
    fn mistakes_are_recorded() {
        let mut game = game_from("4k3/8/4p3/8/8/8/8/3QK3 w - - 0 1", Color::White);
        let turn = game.play("Dd5").unwrap();
        assert_eq!(turn.mistake.unwrap().kind, MistakeKind::HangingPiece);
        assert_eq!(game.mistakes.len(), 1);
        assert_eq!(game.mistakes[0].move_number, 1);
        assert_eq!(game.mistakes[0].played, "Qd5");
    }

    #[test]
    fn mate_ends_the_game_with_a_win() {
        let mut game = game_from("6k1/5ppp/8/8/8/8/8/R3K3 w - - 0 1", Color::White);
        let turn = game.play("Ta8").unwrap();
        assert_eq!(turn.played, "Ra8#");
        assert!(game.is_over());
        assert!(!game.engine_to_move());
        assert_eq!(game.engine_move(), None);
        assert_eq!(game.score(), Some(Score::Win));
        assert_eq!(game.play("Kd1"), Err(MoveError::GameOver));
        assert_eq!(
            game.play_squares(Square::E1, Square::D1),
            Err(MoveError::GameOver)
        );
    }

    #[test]
    fn castling_reports_the_king_squares() {
        let mut game = game_from("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", Color::White);
        game.play("0-0").unwrap();
        assert_eq!(game.last_move(), Some((Square::E1, Square::G1)));
    }

    #[test]
    fn being_mated_is_a_loss() {
        let game = game_from("R5k1/5ppp/8/8/8/8/8/4K3 b - - 0 1", Color::Black);
        assert_eq!(game.score(), Some(Score::Loss));
    }

    #[test]
    fn stalemate_is_a_draw() {
        let game = game_from("7k/8/6Q1/8/8/8/8/K7 b - - 0 1", Color::White);
        assert_eq!(game.score(), Some(Score::Draw));
    }

    #[test]
    fn resigning_is_a_loss_and_recorded_once() {
        let mut game = Game::new(Color::White, 1000.0, 1);
        assert_eq!(game.take_record("t".to_owned()), None);
        game.play("e4").unwrap();
        game.resign();
        assert!(game.is_over());
        let record = game.take_record("2026-11-08".to_owned()).unwrap();
        assert_eq!(record.score, Score::Loss);
        assert!(record.player_white);
        assert_eq!(record.moves.len(), 1);
        assert_eq!(record.played_at, "2026-11-08");
        assert!((record.opponent_rating - 1000.0).abs() < f64::EPSILON);
        assert_eq!(game.take_record("igen".to_owned()), None);
    }

    #[test]
    fn black_record_says_so() {
        let mut game = Game::new(Color::Black, 1000.0, 1);
        game.resign();
        assert!(!game.take_record(String::new()).unwrap().player_white);
    }

    #[test]
    fn describe_uses_the_current_board() {
        let game = Game::new(Color::White, 1000.0, 1);
        assert!(game.describe().ends_with("Vit vid draget."));
    }
}
