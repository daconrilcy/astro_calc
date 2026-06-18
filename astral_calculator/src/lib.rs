//! API publique du crate `astral_calculator`.

pub mod astrology;
/// Bootstrap des variables d'environnement, de la CLI et de la connexion DB.
pub mod bootstrap;
/// Contrats métier purs partagés entre les fonctionnalités.
pub mod domain;
/// Orchestration des calculs et assemblage des réponses runtime.
pub mod engine;
/// Produits exposés par le calculateur: natal, simplified et horoscope.
pub mod features;
/// Adaptateurs d'infrastructure, principalement l'accès PostgreSQL.
pub mod infra;
/// Construction du runtime prêt à exécuter une demande complète.
pub mod runtime;
/// Outils transverses sans dépendance métier produit.
pub mod shared;

pub use engine::engine_request_from_env;

/// Alias historique vers les utilitaires de détection d'aspects.
pub mod aspects {
    pub use crate::astrology::aspects::*;
}

/// Alias historique vers le catalogue natal.
pub mod catalog {
    pub use crate::features::natal::catalog::*;
}

/// Alias historique vers les helpers CLI.
pub mod cli {
    pub use crate::bootstrap::cli::*;
}

/// Alias historique vers le chargement de configuration d'environnement.
pub mod config {
    pub use crate::bootstrap::env::*;
}

/// Alias historique vers le bootstrap de connexion PostgreSQL.
pub mod db {
    pub use crate::bootstrap::db::*;
}

/// Alias historique vers les helpers de dignités natales.
pub mod dignities {
    pub use crate::features::natal::dignities::*;
}

/// Alias historique vers le moteur d'éphémérides.
pub mod ephemeris {
    pub use crate::astrology::ephemeris::*;
}

/// Alias historique vers les helpers de calcul sur les faits astrologiques.
pub mod facts {
    pub use crate::shared::astro_math::*;
}

/// Alias historique vers les primitives d'idempotence.
pub mod idempotency {
    pub use crate::shared::idempotency::*;
}
