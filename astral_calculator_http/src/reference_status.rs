use astral_calculator::config::ephemeris_path_from_env;
use astral_calculator::infra::db::{
    catalog_repository::CatalogRepository, reference_repository::ReferenceRepository,
};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize)]
pub struct ReferenceStatus {
    pub status: String,
    pub checks: ReferenceChecks,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReferenceChecks {
    pub zodiac_signs: bool,
    pub planets: bool,
    pub houses: bool,
    pub aspects: bool,
    pub rulesets: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeris_path: Option<bool>,
}

pub async fn check_reference_status(pool: &PgPool) -> ReferenceStatus {
    let references = ReferenceRepository::new(pool.clone());
    let catalogs = CatalogRepository::new(pool.clone());
    let mut checks = ReferenceChecks {
        zodiac_signs: false,
        planets: false,
        houses: false,
        aspects: false,
        rulesets: false,
        ephemeris_path: Some(ephemeris_files_present()),
    };

    if let Ok(signs) = references.sign_references().await {
        checks.zodiac_signs = signs.len() == 12;
    }

    if let Ok(houses) = references.house_references().await {
        checks.houses = houses.len() == 12;
    }

    if let Ok(aspects) = references.aspect_definitions().await {
        checks.aspects = !aspects.is_empty();
    }

    if let Ok(reference_version_id) = references.default_reference_version_id().await {
        if let Ok(objects) = references.active_chart_objects(reference_version_id).await {
            checks.planets = !objects.is_empty();
        }

        if let Ok(profile) = catalogs
            .basic_product_scoring_profile("basic", "natal_structured_v13")
            .await
        {
            if let Ok(rules) = catalogs
                .essential_dignity_rule_references(
                    reference_version_id,
                    profile.essential_dignity_score_profile_id,
                )
                .await
            {
                checks.rulesets = !rules.is_empty();
            }
        }
    }

    let ready = checks.zodiac_signs
        && checks.planets
        && checks.houses
        && checks.aspects
        && checks.rulesets
        && checks.ephemeris_path.unwrap_or(false);

    ReferenceStatus {
        status: if ready { "ready" } else { "degraded" }.to_string(),
        checks,
    }
}

pub fn ephemeris_files_present() -> bool {
    let path = ephemeris_path_from_env();
    if path.is_file() {
        return path.extension().is_some_and(|ext| ext == "se1");
    }
    if !path.is_dir() {
        return false;
    }
    std::fs::read_dir(&path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|entry| entry.path().extension().is_some_and(|ext| ext == "se1"))
}

pub fn is_ready(status: &ReferenceStatus) -> bool {
    status.status == "ready"
}

pub async fn database_ready(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1").execute(pool).await.is_ok()
}

pub fn reference_check_details(status: &ReferenceStatus) -> Value {
    json!({
        "zodiac_signs": status.checks.zodiac_signs,
        "planets": status.checks.planets,
        "houses": status.checks.houses,
        "aspects": status.checks.aspects,
        "rulesets": status.checks.rulesets,
        "ephemeris_path": status.checks.ephemeris_path.unwrap_or(false),
    })
}

pub fn readiness_report(db_ok: bool, status: &ReferenceStatus) -> Value {
    json!({
        "database": db_ok,
        "reference": reference_check_details(status),
    })
}

pub async fn ensure_ready(pool: &PgPool) -> Result<(), Value> {
    let db_ok = database_ready(pool).await;
    let status = check_reference_status(pool).await;
    if !db_ok || !is_ready(&status) {
        return Err(readiness_report(db_ok, &status));
    }
    Ok(())
}
