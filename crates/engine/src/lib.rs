//! Schackmotor för tränaren, byggd på `cozy-chess`.
//!
//! Reglerna och draggenereringen kommer från `cozy-chess`. Här finns det som
//! tränaren behöver ovanpå: SAN, en sökning med justerbar styrka, bedömning av
//! spelarens fel och en beskrivning av ställningen i ord.

pub mod attacks;
pub mod describe;
pub mod explain;
pub mod mistakes;
pub mod san;
pub mod search;

pub use cozy_chess::{Board, Color, File, GameStatus, Move, Piece, Rank, Square};
