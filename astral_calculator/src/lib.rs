//! API publique du crate `astral_calculator`.

/// Ports applicatifs indépendants des adaptateurs SQL concrets.
pub mod application;
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
#[deprecated(note = "use astral_calculator::astrology::aspects")]
pub mod aspects {
    pub use crate::astrology::aspects::*;
}

/// Alias historique vers le catalogue natal.
#[deprecated(note = "use astral_calculator::features::natal::catalog")]
pub mod catalog {
    pub use crate::features::natal::catalog::*;
}

/// Alias historique vers les helpers CLI.
#[deprecated(note = "use astral_calculator::bootstrap::cli")]
pub mod cli {
    pub use crate::bootstrap::cli::*;
}

/// Alias historique vers le chargement de configuration d'environnement.
#[deprecated(note = "use astral_calculator::bootstrap::env")]
pub mod config {
    pub use crate::bootstrap::env::*;
}

/// Alias historique vers le bootstrap de connexion PostgreSQL.
#[deprecated(note = "use astral_calculator::bootstrap::db")]
pub mod db {
    pub use crate::bootstrap::db::*;
}

/// Alias historique vers les helpers de dignités natales.
#[deprecated(note = "use astral_calculator::features::natal::dignities")]
pub mod dignities {
    pub use crate::features::natal::dignities::*;
}

/// Alias historique vers le moteur d'éphémérides.
#[deprecated(note = "use astral_calculator::astrology::ephemeris")]
pub mod ephemeris {
    pub use crate::astrology::ephemeris::*;
}

/// Alias historique vers les helpers de calcul sur les faits astrologiques.
#[deprecated(note = "use astral_calculator::shared::astro_math")]
pub mod facts {
    pub use crate::shared::astro_math::*;
}

/// Alias historique vers les primitives d'idempotence.
#[deprecated(note = "use astral_calculator::shared::idempotency")]
pub mod idempotency {
    pub use crate::shared::idempotency::*;
}
