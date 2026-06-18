use std::sync::Arc;

use astral_calculator::ephemeris::SwissEphemerisEngine;
use astral_calculator::runtime::ChartCalculationRuntimeService;
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::schema_registry::SchemaRegistry;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub pool: PgPool,
    pub service: Arc<ChartCalculationRuntimeService<SwissEphemerisEngine>>,
    pub schema_registry: Arc<SchemaRegistry>,
}
