//! Utilitaires trigonométriques et zodiacaux communs aux calculs astrologiques.

pub use crate::astrology::angles::{arc_contains, normalize_degrees, shortest_angular_distance};
pub use crate::astrology::zodiac::{whole_sign_house_number, zodiac_slot_for_longitude};
