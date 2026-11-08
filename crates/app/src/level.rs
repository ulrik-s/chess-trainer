//! Motorns nivåer och hur spelarens rating väljer nivå.

use chess_trainer_engine::search::Strength;

/// En nivå: en ungefärlig rating och den motorstyrka som ska motsvara den.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// Nominell rating som används i Elo-uppskattningen.
    pub rating: f64,
    /// Motorns sökdjup och brus.
    pub strength: Strength,
}

/// Kortform för att skriva nivåtabellen.
const fn level(rating: f64, depth: u8, noise: i32) -> Level {
    Level {
        rating,
        strength: Strength { depth, noise },
    }
}

/// Alla nivåer, svagast först.
// ponytail: de nominella ratingarna är gissningar. Kalibreringsratten: spela mot chess.com-botar
// på känd nivå och justera talen tills resultaten stämmer.
pub const LEVELS: [Level; 7] = [
    level(400.0, 1, 400),
    level(600.0, 1, 200),
    level(800.0, 2, 150),
    level(1000.0, 2, 80),
    level(1200.0, 3, 60),
    level(1400.0, 3, 25),
    level(1600.0, 3, 0),
];

/// Nivån närmast `rating`.
#[must_use]
pub fn level_for(rating: f64) -> Level {
    LEVELS
        .iter()
        .copied()
        .min_by(|a, b| {
            (a.rating - rating)
                .abs()
                .total_cmp(&(b.rating - rating).abs())
        })
        .unwrap_or(level(1000.0, 2, 80))
}

#[cfg(test)]
#[allow(
    clippy::missing_docs_in_private_items,
    reason = "testnamnen är dokumentationen"
)]
mod tests {
    use super::{LEVELS, Strength, level_for};

    #[test]
    fn levels_get_stronger() {
        assert!(LEVELS.windows(2).all(|w| w[0].rating < w[1].rating));
        assert!(
            LEVELS
                .windows(2)
                .all(|w| w[0].strength.noise >= w[1].strength.noise)
        );
        assert!(
            LEVELS
                .windows(2)
                .all(|w| w[0].strength.depth <= w[1].strength.depth)
        );
    }

    #[test]
    fn picks_nearest_level() {
        assert!((level_for(1000.0).rating - 1000.0).abs() < f64::EPSILON);
        assert!((level_for(1090.0).rating - 1000.0).abs() < f64::EPSILON);
        assert!((level_for(1110.0).rating - 1200.0).abs() < f64::EPSILON);
        assert_eq!(
            level_for(1110.0).strength,
            Strength {
                depth: 3,
                noise: 60
            }
        );
    }

    #[test]
    fn clamps_at_the_ends() {
        assert!((level_for(0.0).rating - 400.0).abs() < f64::EPSILON);
        assert!((level_for(3000.0).rating - 1600.0).abs() < f64::EPSILON);
    }
}
