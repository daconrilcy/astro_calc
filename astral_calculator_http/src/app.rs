//! Assemble l'application HTTP avec les couches transverses appliquees au routeur.
//! Ce module ne porte pas de logique metier: il compose le routage, les limites de
//! requetes, le timeout et la verification de cle API autour de `routes`.

use std::time::Duration;

use axum::http::StatusCode;
use axum::Router;
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer, trace::TraceLayer};

use crate::auth::require_api_key;
use crate::routes;
use crate::state::AppState;

/// Construit le routeur HTTP complet a partir de l'etat partage.
/// L'ordre des couches est volontairement centralise ici pour garder le point
/// d'entree du serveur lisible et facile a auditer.
pub fn build_app(state: AppState) -> Router {
    let timeout = Duration::from_millis(state.config.request_timeout_ms);
    routes::router(state.clone())
        .layer(RequestBodyLimitLayer::new(state.config.max_body_bytes))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::GATEWAY_TIMEOUT,
            timeout,
        ))
        .layer(axum::middleware::from_fn_with_state(state, require_api_key))
        .layer(TraceLayer::new_for_http())
}
