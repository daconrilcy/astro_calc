//! Primitives astrologiques réutilisables indépendantes des produits `natal`,
//! `simplified` et `horoscope`.

pub mod aspects;
/// Calcul des positions, maisons et angles à partir d'un moteur d'éphémérides.
pub mod ephemeris;
/// Géométrie des maisons à partir des cuspides calculées.
pub mod house_geometry;
/// Calculs réutilisables de transits et d'aspects transit-vers-natal.
pub mod transits;
