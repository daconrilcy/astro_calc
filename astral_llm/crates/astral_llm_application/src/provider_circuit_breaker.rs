use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use astral_llm_domain::ProviderKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
struct ProviderCircuit {
    state: CircuitBreakerState,
    consecutive_failures: u32,
    open_until: Option<Instant>,
    half_open_probe_active: bool,
}

#[derive(Debug)]
pub struct ProviderCircuitBreaker {
    failure_threshold: u32,
    open_duration: Duration,
    inner: Mutex<HashMap<String, ProviderCircuit>>,
}

impl ProviderCircuitBreaker {
    pub fn new(failure_threshold: u32, open_duration_secs: u64) -> Self {
        Self {
            failure_threshold: failure_threshold.max(1),
            open_duration: Duration::from_secs(open_duration_secs),
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn allows_call(&self, provider: &ProviderKind) -> bool {
        let key = provider.as_str().to_string();
        let mut guard = self.inner.lock().expect("circuit breaker lock");
        let circuit = guard.entry(key).or_insert_with(|| ProviderCircuit {
            state: CircuitBreakerState::Closed,
            consecutive_failures: 0,
            open_until: None,
            half_open_probe_active: false,
        });

        match circuit.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::HalfOpen => {
                if circuit.half_open_probe_active {
                    false
                } else {
                    circuit.half_open_probe_active = true;
                    true
                }
            }
            CircuitBreakerState::Open => {
                if let Some(until) = circuit.open_until {
                    if Instant::now() >= until {
                        circuit.state = CircuitBreakerState::HalfOpen;
                        circuit.half_open_probe_active = true;
                        return true;
                    }
                }
                false
            }
        }
    }

    pub fn record_success(&self, provider: &ProviderKind) {
        let key = provider.as_str().to_string();
        let mut guard = self.inner.lock().expect("circuit breaker lock");
        if let Some(circuit) = guard.get_mut(&key) {
            circuit.state = CircuitBreakerState::Closed;
            circuit.consecutive_failures = 0;
            circuit.open_until = None;
            circuit.half_open_probe_active = false;
        }
    }

    pub fn record_transient_failure(&self, provider: &ProviderKind) {
        let key = provider.as_str().to_string();
        let mut guard = self.inner.lock().expect("circuit breaker lock");
        let circuit = guard.entry(key).or_insert_with(|| ProviderCircuit {
            state: CircuitBreakerState::Closed,
            consecutive_failures: 0,
            open_until: None,
            half_open_probe_active: false,
        });

        circuit.half_open_probe_active = false;

        if circuit.state == CircuitBreakerState::HalfOpen {
            circuit.state = CircuitBreakerState::Open;
            circuit.open_until = Some(Instant::now() + self.open_duration);
            circuit.consecutive_failures = self.failure_threshold;
            return;
        }

        circuit.consecutive_failures = circuit.consecutive_failures.saturating_add(1);
        if circuit.consecutive_failures >= self.failure_threshold {
            circuit.state = CircuitBreakerState::Open;
            circuit.open_until = Some(Instant::now() + self.open_duration);
        }
    }

    pub fn release_half_open_probe(&self, provider: &ProviderKind) {
        let key = provider.as_str().to_string();
        let mut guard = self.inner.lock().expect("circuit breaker lock");
        if let Some(circuit) = guard.get_mut(&key) {
            circuit.half_open_probe_active = false;
        }
    }

    pub fn snapshot(&self) -> Vec<(String, CircuitBreakerState)> {
        let guard = self.inner.lock().expect("circuit breaker lock");
        guard.iter().map(|(k, c)| (k.clone(), c.state)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_after_threshold() {
        let cb = ProviderCircuitBreaker::new(2, 30);
        let provider = ProviderKind::OpenAi;
        assert!(cb.allows_call(&provider));
        cb.record_transient_failure(&provider);
        assert!(cb.allows_call(&provider));
        cb.record_transient_failure(&provider);
        assert!(!cb.allows_call(&provider));
    }

    #[test]
    fn half_open_allows_single_probe() {
        let cb = ProviderCircuitBreaker::new(1, 60);
        let provider = ProviderKind::Mistral;
        cb.record_transient_failure(&provider);
        assert!(!cb.allows_call(&provider));
        // simulate cooldown elapsed by forcing half-open
        {
            let mut guard = cb.inner.lock().unwrap();
            let circuit = guard.get_mut("mistral").unwrap();
            circuit.state = CircuitBreakerState::HalfOpen;
            circuit.half_open_probe_active = false;
            circuit.open_until = None;
        }
        assert!(cb.allows_call(&provider));
        assert!(!cb.allows_call(&provider));
        cb.record_success(&provider);
        assert!(cb.allows_call(&provider));
    }
}
