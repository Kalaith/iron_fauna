//! Small shared utilities.

/// Deterministic RNG for gameplay — re-exported from macroquad-toolkit so
/// randomness stays seeded, state-owned, and shared across games
/// (`CODE_STANDARDS.md`: keep gameplay deterministic).
pub use macroquad_toolkit::rng::SeededRng as Rng;
