use chrono::{DateTime, Duration, Utc};

use super::catalog::SimplifiedCatalog;

pub fn sample_points_utc(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    sampling_minutes: i32,
) -> Vec<DateTime<Utc>> {
    let step = Duration::minutes(i64::from(sampling_minutes.max(1)));
    let mut points = Vec::new();
    points.push(start);

    let mut cursor = start + step;
    while cursor < end {
        points.push(cursor);
        cursor += step;
    }

    if points.last().copied() != Some(end) {
        points.push(end);
    }

    points
}

pub fn build_sampling_schedule(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    catalog: &SimplifiedCatalog,
) -> Vec<DateTime<Utc>> {
    sample_points_utc(start, end, catalog.policy.uncertainty_sampling_minutes)
}
