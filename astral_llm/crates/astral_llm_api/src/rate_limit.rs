use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    extract::State,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::sync::Semaphore;

use crate::state::AppState;

#[derive(Debug)]
struct KeyLimiterState {
    concurrent: u32,
    premium_concurrent: u32,
    minute_timestamps: VecDeque<Instant>,
}

#[derive(Debug, Clone)]
pub struct ApiKeyRateLimiter {
    max_concurrent_per_key: usize,
    max_requests_per_minute: u32,
    max_premium_concurrent: u32,
    inner: Arc<Mutex<HashMap<String, KeyLimiterState>>>,
}

impl ApiKeyRateLimiter {
    pub fn new(
        max_concurrent_per_key: usize,
        max_requests_per_minute: u32,
        max_premium_concurrent: u32,
    ) -> Self {
        Self {
            max_concurrent_per_key,
            max_requests_per_minute,
            max_premium_concurrent,
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn try_acquire(&self, key_id: &str, is_premium: bool) -> Result<ApiKeyPermit<'_>, RateLimitReason> {
        let mut guard = self.inner.lock().expect("rate limiter lock");
        let state = guard
            .entry(key_id.to_string())
            .or_insert_with(|| KeyLimiterState {
                concurrent: 0,
                premium_concurrent: 0,
                minute_timestamps: VecDeque::new(),
            });

        let now = Instant::now();
        let window_start = now - Duration::from_secs(60);
        while state
            .minute_timestamps
            .front()
            .is_some_and(|t| *t < window_start)
        {
            state.minute_timestamps.pop_front();
        }

        if state.minute_timestamps.len() as u32 >= self.max_requests_per_minute {
            return Err(RateLimitReason::RequestsPerMinute);
        }

        if state.concurrent as usize >= self.max_concurrent_per_key {
            return Err(RateLimitReason::ConcurrentPerKey);
        }

        if is_premium
            && self.max_premium_concurrent > 0
            && state.premium_concurrent >= self.max_premium_concurrent
        {
            return Err(RateLimitReason::PremiumConcurrent);
        }

        state.minute_timestamps.push_back(now);
        state.concurrent += 1;
        if is_premium {
            state.premium_concurrent += 1;
        }

        Ok(ApiKeyPermit {
            limiter: self,
            key_id: key_id.to_string(),
            is_premium,
        })
    }
}

pub struct ApiKeyPermit<'a> {
    limiter: &'a ApiKeyRateLimiter,
    key_id: String,
    is_premium: bool,
}

impl Drop for ApiKeyPermit<'_> {
    fn drop(&mut self) {
        let mut guard = self.limiter.inner.lock().expect("rate limiter lock");
        if let Some(state) = guard.get_mut(&self.key_id) {
            state.concurrent = state.concurrent.saturating_sub(1);
            if self.is_premium {
                state.premium_concurrent = state.premium_concurrent.saturating_sub(1);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RateLimitReason {
    ConcurrentPerKey,
    RequestsPerMinute,
    PremiumConcurrent,
}

pub async fn api_key_rate_limit(
    State(state): State<AppState>,
    request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if path == "/health" {
        return next.run(request).await;
    }

    let Some(limiter) = state.api_key_limiter.as_ref() else {
        return next.run(request).await;
    };

    let key_id = rate_limit_key_id(&request, &state);
    let permit = match limiter.try_acquire(&key_id, false) {
        Ok(permit) => permit,
        Err(reason) => return rate_limit_response(reason).into_response(),
    };
    let response = next.run(request).await;
    drop(permit);
    response
}

pub fn rate_limit_key_id_from_headers(
    headers: &axum::http::HeaderMap,
    state: &AppState,
) -> String {
    if let Some(token) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        return format!("key:{}", astral_llm_infra::hash_json(&serde_json::json!(token)));
    }
    if let Some(token) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        return format!("key:{}", astral_llm_infra::hash_json(&serde_json::json!(token)));
    }
    if state.config.requires_auth() {
        "key:authenticated".into()
    } else {
        "key:anonymous".into()
    }
}

pub fn try_acquire_premium_addon<'a>(
    state: &'a AppState,
    key_id: &str,
) -> Result<PremiumAddonPermit<'a>, RateLimitReason> {
    let limiter = state
        .api_key_limiter
        .as_ref()
        .ok_or(RateLimitReason::PremiumConcurrent)?;
    limiter.reserve_premium_slot(key_id)
}

#[derive(Debug)]
pub struct PremiumAddonPermit<'a> {
    limiter: &'a ApiKeyRateLimiter,
    key_id: String,
}

impl ApiKeyRateLimiter {
    pub fn reserve_premium_slot(&self, key_id: &str) -> Result<PremiumAddonPermit<'_>, RateLimitReason> {
        let mut guard = self.inner.lock().expect("rate limiter lock");
        let state = guard
            .entry(key_id.to_string())
            .or_insert_with(|| KeyLimiterState {
                concurrent: 0,
                premium_concurrent: 0,
                minute_timestamps: VecDeque::new(),
            });

        if self.max_premium_concurrent > 0
            && state.premium_concurrent >= self.max_premium_concurrent
        {
            return Err(RateLimitReason::PremiumConcurrent);
        }
        state.premium_concurrent += 1;
        Ok(PremiumAddonPermit {
            limiter: self,
            key_id: key_id.to_string(),
        })
    }
}

impl Drop for PremiumAddonPermit<'_> {
    fn drop(&mut self) {
        let mut guard = self.limiter.inner.lock().expect("rate limiter lock");
        if let Some(state) = guard.get_mut(&self.key_id) {
            state.premium_concurrent = state.premium_concurrent.saturating_sub(1);
        }
    }
}

pub fn rate_limit_key_id(request: &axum::http::Request<axum::body::Body>, state: &AppState) -> String {
    if let Some(token) = extract_api_token(request) {
        return format!("key:{}", astral_llm_infra::hash_json(&serde_json::json!(token)));
    }
    if state.config.requires_auth() {
        "key:authenticated".into()
    } else {
        "key:anonymous".into()
    }
}

fn extract_api_token(request: &axum::http::Request<axum::body::Body>) -> Option<String> {
    request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string)
        .or_else(|| {
            request
                .headers()
                .get("x-api-key")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        })
}

fn rate_limit_response(reason: RateLimitReason) -> (StatusCode, axum::Json<serde_json::Value>) {
    let message = match reason {
        RateLimitReason::ConcurrentPerKey => "API key concurrent request limit reached",
        RateLimitReason::RequestsPerMinute => "API key requests per minute limit reached",
        RateLimitReason::PremiumConcurrent => "API key premium concurrent limit reached",
    };
    (
        StatusCode::TOO_MANY_REQUESTS,
        axum::Json(serde_json::json!({
            "error": "too_many_requests",
            "message": message
        })),
    )
}

pub async fn concurrency_limit(
    State(state): State<AppState>,
    request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if path == "/health" {
        return next.run(request).await;
    }

    let Some(semaphore) = state.concurrency_limit.as_ref() else {
        return next.run(request).await;
    };

    let permit = match semaphore.try_acquire() {
        Ok(permit) => permit,
        Err(_) => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                axum::Json(serde_json::json!({
                    "error": "too_many_requests",
                    "message": "server concurrency limit reached"
                })),
            )
                .into_response();
        }
    };

    let response = next.run(request).await;
    drop(permit);
    response
}

pub fn new_semaphore(max_concurrent: usize) -> Option<Arc<Semaphore>> {
    if max_concurrent == 0 {
        None
    } else {
        Some(Arc::new(Semaphore::new(max_concurrent)))
    }
}

pub fn new_api_key_limiter(config: &astral_llm_infra::AppConfig) -> Option<Arc<ApiKeyRateLimiter>> {
    if config.max_concurrent_requests_per_key == 0 {
        return None;
    }
    Some(Arc::new(ApiKeyRateLimiter::new(
        config.max_concurrent_requests_per_key,
        config.max_requests_per_minute_per_key,
        config.max_premium_runs_per_key,
    )))
}
