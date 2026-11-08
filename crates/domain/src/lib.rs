//! Domänlogik för schacktränaren.
//!
//! Lagret är rent: inga beroenden, ingen I/O, inga sidoeffekter. Allt som
//! pratar med motor, lagring eller användare ligger i andra crates.

pub mod elo;
pub mod notation;
