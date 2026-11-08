//! Kunskapsbasen: alla partier, felen i dem och den uppskattade ratingen.
//!
//! Sparas som JSON i webbläsarens `localStorage`. Formatet är versionslöst tills det behövs.

use chess_trainer_domain::elo::{Outcome, updated_rating};
use chess_trainer_engine::mistakes::MistakeKind;
use serde::{Deserialize, Serialize};

/// Startrating. Från chatten: klarar normalt chess.coms bot på 1000.
pub const START_RATING: f64 = 1000.0;

/// Hur många av de senaste partierna lärdomarna bygger på.
const LESSON_WINDOW: usize = 10;

/// Högst så många lärdomar visas åt gången. Chatten sa två till tre.
const MAX_LESSONS: usize = 3;

/// Resultat ur spelarens perspektiv.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Score {
    /// Vinst.
    Win,
    /// Remi.
    Draw,
    /// Förlust, inklusive uppgivet parti.
    Loss,
}

impl Score {
    /// Motsvarande Elo-utfall.
    #[must_use]
    pub const fn outcome(self) -> Outcome {
        match self {
            Self::Win => Outcome::Win,
            Self::Draw => Outcome::Draw,
            Self::Loss => Outcome::Loss,
        }
    }
}

/// Ett fel i ett sparat parti.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MistakeRecord {
    /// Dragnummer.
    pub move_number: u16,
    /// Spelat drag i SAN.
    pub played: String,
    /// Bästa draget i SAN.
    pub best: String,
    /// Vilken sorts fel.
    pub kind: MistakeKind,
}

/// Ett avslutat parti.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameRecord {
    /// När partiet spelades (ISO 8601 från webbläsaren).
    pub played_at: String,
    /// Spelade spelaren vit?
    pub player_white: bool,
    /// Alla drag i SAN.
    pub moves: Vec<String>,
    /// Resultatet.
    pub score: Score,
    /// Motorns nominella rating.
    pub opponent_rating: f64,
    /// Spelarens uppskattade rating efter partiet. Sätts av [`KnowledgeBase::add`].
    pub rating_after: f64,
    /// Felen spelaren gjorde.
    pub mistakes: Vec<MistakeRecord>,
}

/// Allt tränaren vet om spelaren.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeBase {
    /// Uppskattad rating.
    pub rating: f64,
    /// Alla partier, äldst först.
    pub games: Vec<GameRecord>,
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self {
            rating: START_RATING,
            games: Vec::new(),
        }
    }
}

/// Råd för en feltyp, formulerat som rutinen från chatten.
#[must_use]
pub const fn advice(kind: MistakeKind) -> &'static str {
    match kind {
        MistakeKind::HangingPiece => {
            "Dragkontroll: innan du släpper pjäsen, kolla om den kan slås gratis på sin nya ruta."
        }
        MistakeKind::MissedCapture => {
            "Leta efter slag först: kan du ta en pjäs gratis eller vinna material?"
        }
        MistakeKind::MissedThreat => "Kolla motståndarens hot: vad vill det senaste draget göra?",
    }
}

/// "1 gång" eller "3 gånger".
fn times(count: usize) -> String {
    if count == 1 {
        "1 gång".to_owned()
    } else {
        format!("{count} gånger")
    }
}

impl KnowledgeBase {
    /// Läser kunskapsbasen från JSON.
    ///
    /// # Errors
    /// Om texten inte är en giltig kunskapsbas.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Kunskapsbasen som JSON.
    ///
    /// # Errors
    /// Om serialiseringen misslyckas, vilket inte ska kunna hända för de här typerna.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Lägger till ett avslutat parti och uppdaterar ratingen.
    pub fn add(&mut self, mut record: GameRecord) {
        self.rating = updated_rating(self.rating, record.opponent_rating, record.score.outcome());
        record.rating_after = self.rating;
        self.games.push(record);
    }

    /// De viktigaste lärdomarna från de senaste partierna, vanligaste felet först.
    #[must_use]
    pub fn lessons(&self) -> Vec<String> {
        if self.games.is_empty() {
            return vec!["Spela ett parti så börjar jag samla lärdomar.".to_owned()];
        }
        let recent: Vec<&GameRecord> = self.games.iter().rev().take(LESSON_WINDOW).collect();
        let window = if recent.len() == 1 {
            "senaste partiet".to_owned()
        } else {
            format!("de senaste {} partierna", recent.len())
        };
        let mut counts: Vec<(MistakeKind, usize)> = [
            MistakeKind::HangingPiece,
            MistakeKind::MissedCapture,
            MistakeKind::MissedThreat,
        ]
        .into_iter()
        .map(|kind| {
            let count = recent
                .iter()
                .flat_map(|game| &game.mistakes)
                .filter(|mistake| mistake.kind == kind)
                .count();
            (kind, count)
        })
        .filter(|(_, count)| *count > 0)
        .collect();
        if counts.is_empty() {
            return vec![format!("Inga tydliga fel i {window}. Snyggt!")];
        }
        counts.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        counts
            .into_iter()
            .take(MAX_LESSONS)
            .map(|(kind, count)| format!("{} ({} i {window})", advice(kind), times(count)))
            .collect()
    }
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    reason = "testnamnen är dokumentationen"
)]
mod tests {
    use super::{
        GameRecord, KnowledgeBase, MistakeKind, MistakeRecord, Outcome, START_RATING, Score, advice,
    };

    fn mistake(kind: MistakeKind) -> MistakeRecord {
        MistakeRecord {
            move_number: 5,
            played: "Qd5".to_owned(),
            best: "Qd2".to_owned(),
            kind,
        }
    }

    fn record(score: Score, mistakes: Vec<MistakeRecord>) -> GameRecord {
        GameRecord {
            played_at: "2026-11-08T12:00:00Z".to_owned(),
            player_white: true,
            moves: vec!["e4".to_owned(), "e5".to_owned()],
            score,
            opponent_rating: 1000.0,
            rating_after: 0.0,
            mistakes,
        }
    }

    #[test]
    fn score_maps_to_outcome() {
        assert_eq!(Score::Win.outcome(), Outcome::Win);
        assert_eq!(Score::Draw.outcome(), Outcome::Draw);
        assert_eq!(Score::Loss.outcome(), Outcome::Loss);
    }

    #[test]
    fn default_starts_at_one_thousand() {
        let kb = KnowledgeBase::default();
        assert!((kb.rating - START_RATING).abs() < f64::EPSILON);
        assert!((START_RATING - 1000.0).abs() < f64::EPSILON);
        assert!(kb.games.is_empty());
    }

    #[test]
    fn add_updates_rating_and_record() {
        let mut kb = KnowledgeBase::default();
        kb.add(record(Score::Win, vec![]));
        assert!((kb.rating - 1016.0).abs() < f64::EPSILON);
        assert!((kb.games[0].rating_after - 1016.0).abs() < f64::EPSILON);
        kb.add(record(Score::Loss, vec![]));
        assert!(kb.rating < 1016.0);
        assert_eq!(kb.games.len(), 2);
    }

    #[test]
    fn json_round_trip() {
        let mut kb = KnowledgeBase::default();
        kb.add(record(
            Score::Draw,
            vec![mistake(MistakeKind::HangingPiece)],
        ));
        let json = kb.to_json().unwrap();
        assert!(json.contains("HangingPiece"));
        assert_eq!(KnowledgeBase::from_json(&json).unwrap(), kb);
    }

    #[test]
    fn broken_json_is_an_error() {
        assert!(KnowledgeBase::from_json("{inte json").is_err());
    }

    #[test]
    fn advice_differs_per_kind() {
        assert!(advice(MistakeKind::HangingPiece).starts_with("Dragkontroll"));
        assert!(advice(MistakeKind::MissedCapture).starts_with("Leta efter slag"));
        assert!(advice(MistakeKind::MissedThreat).starts_with("Kolla motståndarens hot"));
    }

    #[test]
    fn lessons_before_any_game() {
        let lessons = KnowledgeBase::default().lessons();
        assert_eq!(
            lessons,
            vec!["Spela ett parti så börjar jag samla lärdomar."]
        );
    }

    #[test]
    fn lessons_without_mistakes() {
        let mut kb = KnowledgeBase::default();
        kb.add(record(Score::Win, vec![]));
        assert_eq!(
            kb.lessons(),
            vec!["Inga tydliga fel i senaste partiet. Snyggt!"]
        );
        kb.add(record(Score::Win, vec![]));
        assert_eq!(
            kb.lessons(),
            vec!["Inga tydliga fel i de senaste 2 partierna. Snyggt!"]
        );
    }

    #[test]
    fn lessons_are_sorted_by_frequency_and_capped() {
        let mut kb = KnowledgeBase::default();
        kb.add(record(
            Score::Loss,
            vec![
                mistake(MistakeKind::MissedThreat),
                mistake(MistakeKind::HangingPiece),
                mistake(MistakeKind::HangingPiece),
                mistake(MistakeKind::MissedCapture),
                mistake(MistakeKind::MissedCapture),
                mistake(MistakeKind::MissedCapture),
            ],
        ));
        let lessons = kb.lessons();
        assert_eq!(lessons.len(), 3);
        assert!(lessons[0].starts_with("Leta efter slag"));
        assert!(lessons[0].ends_with("(3 gånger i senaste partiet)"));
        assert!(lessons[1].starts_with("Dragkontroll"));
        assert!(lessons[2].starts_with("Kolla motståndarens hot"));
        assert!(lessons[2].ends_with("(1 gång i senaste partiet)"));
    }

    #[test]
    fn lessons_only_look_at_the_last_ten_games() {
        let mut kb = KnowledgeBase::default();
        kb.add(record(
            Score::Loss,
            vec![mistake(MistakeKind::HangingPiece)],
        ));
        for _ in 0..10 {
            kb.add(record(Score::Win, vec![]));
        }
        assert_eq!(
            kb.lessons(),
            vec!["Inga tydliga fel i de senaste 10 partierna. Snyggt!"]
        );
    }
}
