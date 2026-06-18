//! Contient l'etat partage du serveur HTTP utilise par Axum.
//! Le state regroupe la configuration, le pool PostgreSQL, le runtime et le
//! registre de schemas pour les handlers.

use std::sync::Arc;

use astral_calculator::ephemeris::SwissEphemerisEngine;
use astral_calculator::runtime::ChartCalculationRuntimeService;
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::schema_registry::SchemaRegistry;

/// Etat partage entre tous les handlers HTTP.
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub pool: PgPool,
    pub service: Arc<ChartCalculationRuntimeService<SwissEphemerisEngine>>,
    pub schema_registry: Arc<SchemaRegistry>,
}
