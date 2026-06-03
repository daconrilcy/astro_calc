use std::time::Duration;

use tokio::time::timeout;

use crate::LlmProviderError;

pub async fn with_timeout<T, F>(limit: Duration, fut: F) -> Result<T, LlmProviderError>
where
    F: std::future::Future<Output = Result<T, LlmProviderError>>,
{
    timeout(limit, fut)
        .await
        .map_err(|_| LlmProviderError::Timeout)?
}
