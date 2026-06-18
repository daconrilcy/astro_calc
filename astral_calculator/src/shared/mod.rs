//! Briques utilitaires transverses réutilisées dans le crate.

pub mod astro_math;
/// Type d'erreur runtime partagé.
pub mod error;
/// Calcul de signatures stables et de clés d'idempotence.
pub mod idempotency;
/// Helpers temporels indépendants des features produit.
pub mod time;
