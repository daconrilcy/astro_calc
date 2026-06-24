use astral_llm_infra::{NatalExplanationCacheKey, NatalExplanationCacheRecord, RunPersistence};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

async fn test_persistence() -> RunPersistence {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("connect PostgreSQL");
    let persistence = RunPersistence::new(pool);
    persistence.ensure_schema().await.expect("schema");
    persistence
}

fn record(language_code: &str, key_hash: &str, explanation: &str) -> NatalExplanationCacheRecord {
    NatalExplanationCacheRecord {
        language_code: language_code.into(),
        kind_code: "placement".into(),
        key_hash: key_hash.into(),
        key_json: serde_json::json!({
            "type": "placement",
            "object": "sun",
            "sign": "taurus",
            "test_nonce": key_hash
        }),
        title: format!("Title {language_code}"),
        explanation: explanation.into(),
        expression_primary: Some("Maison 10".into()),
        provider: "test".into(),
        model: "test-model".into(),
        prompt_version: "test-v1".into(),
    }
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and PostgreSQL schema access"]
async fn natal_explanation_persistence_separates_fact_from_language_translations() {
    let persistence = test_persistence().await;
    let key_hash = format!("test-natal-explanation-{}", Uuid::new_v4());

    persistence
        .upsert_natal_explanations(&[
            record("fr", &key_hash, "Explication francaise."),
            record("en", &key_hash, "English explanation."),
        ])
        .await
        .expect("upsert translations");

    let fr = persistence
        .lookup_natal_explanations(&[NatalExplanationCacheKey {
            language_code: "fr".into(),
            key_hash: key_hash.clone(),
        }])
        .await
        .expect("lookup fr");
    assert_eq!(fr.len(), 1);
    assert_eq!(fr[0].language_code, "fr");
    assert_eq!(fr[0].explanation, "Explication francaise.");

    let de = persistence
        .lookup_natal_explanations(&[NatalExplanationCacheKey {
            language_code: "de".into(),
            key_hash: key_hash.clone(),
        }])
        .await
        .expect("lookup de");
    assert!(
        de.is_empty(),
        "missing de translation must remain a cache miss"
    );

    let fact_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM llm_natal_explanation_facts WHERE key_hash = $1")
            .bind(&key_hash)
            .fetch_one(persistence.pool())
            .await
            .expect("fact count");
    assert_eq!(fact_count, 1);

    let translation_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM llm_natal_explanation_translations translations
        JOIN llm_natal_explanation_facts facts ON facts.id = translations.fact_id
        WHERE facts.key_hash = $1
        "#,
    )
    .bind(&key_hash)
    .fetch_one(persistence.pool())
    .await
    .expect("translation count");
    assert_eq!(translation_count, 2);
}
