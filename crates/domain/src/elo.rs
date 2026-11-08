//! Grov Elo-uppskattning enligt den klassiska formeln.
//!
//! Ingen riktig rating, bara en skattning som följer resultat mot en motor
//! med känd styrka.

/// K-faktor: hur mycket ett enskilt parti får flytta ratingen.
// ponytail: fast K. FIDE-skalad K (40/20/10) om skattningen någon gång blir för hoppig.
pub const K_FACTOR: f64 = 32.0;

/// Resultat ur spelarens perspektiv.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Spelaren vann.
    Win,
    /// Remi.
    Draw,
    /// Spelaren förlorade.
    Loss,
}

impl Outcome {
    /// Poäng enligt Elo: 1, ½ eller 0.
    #[must_use]
    pub const fn score(self) -> f64 {
        match self {
            Self::Win => 1.0,
            Self::Draw => 0.5,
            Self::Loss => 0.0,
        }
    }
}

/// Förväntad poäng (0–1) för `rating` mot `opponent`.
#[must_use]
pub fn expected_score(rating: f64, opponent: f64) -> f64 {
    1.0 / (1.0 + 10.0_f64.powf((opponent - rating) / 400.0))
}

/// Ny rating efter ett parti, avrundad till heltal.
#[must_use]
pub fn updated_rating(rating: f64, opponent: f64, result: Outcome) -> f64 {
    K_FACTOR
        .mul_add(result.score() - expected_score(rating, opponent), rating)
        .round()
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    reason = "testnamnen är dokumentationen"
)]
mod tests {
    use super::{K_FACTOR, Outcome, expected_score, updated_rating};

    #[test]
    fn score_follows_elo_convention() {
        assert!((Outcome::Win.score() - 1.0).abs() < f64::EPSILON);
        assert!((Outcome::Draw.score() - 0.5).abs() < f64::EPSILON);
        assert!((Outcome::Loss.score() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn equal_ratings_expect_half() {
        assert!((expected_score(1000.0, 1000.0) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn two_hundred_points_higher_expects_about_three_quarters() {
        let expected = expected_score(1200.0, 1000.0);
        assert!((expected - 0.759_746).abs() < 1e-6, "{expected}");
    }

    #[test]
    fn lower_rated_player_expects_less_than_half() {
        assert!(expected_score(1000.0, 1200.0) < 0.5);
    }

    #[test]
    fn k_factor_is_thirty_two() {
        assert!((K_FACTOR - 32.0).abs() < f64::EPSILON);
    }

    #[test]
    fn win_against_equal_gives_plus_sixteen() {
        assert!((updated_rating(1000.0, 1000.0, Outcome::Win) - 1016.0).abs() < f64::EPSILON);
    }

    #[test]
    fn loss_against_equal_gives_minus_sixteen() {
        assert!((updated_rating(1000.0, 1000.0, Outcome::Loss) - 984.0).abs() < f64::EPSILON);
    }

    #[test]
    fn draw_against_equal_changes_nothing() {
        assert!((updated_rating(1000.0, 1000.0, Outcome::Draw) - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn win_against_stronger_is_rounded_to_whole_points() {
        // 1000 + 32 * (1 - 0.2403) = 1024.3 -> 1024
        assert!((updated_rating(1000.0, 1200.0, Outcome::Win) - 1024.0).abs() < f64::EPSILON);
    }

    #[test]
    fn loss_against_weaker_costs_more_than_sixteen() {
        // 1200 + 32 * (0 - 0.7597) = 1175.7 -> 1176
        assert!((updated_rating(1200.0, 1000.0, Outcome::Loss) - 1176.0).abs() < f64::EPSILON);
    }
}
