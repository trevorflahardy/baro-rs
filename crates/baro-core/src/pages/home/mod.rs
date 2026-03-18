//! Home page variants
//!
//! Two distinct home page implementations for different use cases:
//!
//! - **Outdoor** (`outdoor.rs`): Compact status-first dashboard with a quality
//!   banner and priority-sorted sensor rows. Designed for backpack glanceability
//!   on the trail.
//!
//! - **Grid** (`grid.rs`): 2×2 mini-graph grid with auto-cycling through
//!   full-page trend views. Designed for stationary indoor use where the
//!   device sits on a shelf and cycles through data automatically.

pub mod grid;
pub mod outdoor;

pub use grid::HomeGridPage;
pub use outdoor::HomePage;
