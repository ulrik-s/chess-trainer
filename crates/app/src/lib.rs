//! Användningsfallen i schacktränaren.
//!
//! Allt här är rent och testbart: ett parti mot motorn, nivån som följer spelarens
//! rating, kunskapsbasen som sparas i webbläsaren och HTML-vyerna. Webbcratet
//! kopplar bara ihop det här med DOM och `localStorage`.

pub mod game;
pub mod knowledge;
pub mod level;
pub mod view;

pub use chess_trainer_domain::notation::to_swedish;
pub use chess_trainer_engine::{Color, Square};
