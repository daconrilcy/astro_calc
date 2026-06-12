use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PeriodWindowRequest {
    pub period_profile_code: String,
    pub anchor_date: String,
    pub timezone: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_start_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedPeriodWindow {
    pub start_datetime_local: NaiveDateTime,
    pub end_datetime_local: NaiveDateTime,
    pub timezone: String,
    pub duration_days: i64,
    pub end_exclusive: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub included_days: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeriodProfileDefinition {
    pub period_profile_code: String,
    pub resolution_strategy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_days: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub week_offset: Option<i64>,
    #[serde(default)]
    pub included_days: Vec<String>,
    #[serde(default = "default_enabled")]
    pub is_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct PeriodWindowResolver {
    profiles: HashMap<String, PeriodProfileDefinition>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PeriodWindowError {
    #[error("unknown period profile code: {0}")]
    UnknownProfile(String),
    #[error("period profile is disabled: {0}")]
    DisabledProfile(String),
    #[error("invalid date for {field}: {value}")]
    InvalidDate { field: &'static str, value: String },
    #[error("invalid timezone: {0}")]
    InvalidTimezone(String),
    #[error("ambiguous local datetime for {field} in timezone {timezone}")]
    AmbiguousLocalDateTime {
        field: &'static str,
        timezone: String,
    },
    #[error("nonexistent local datetime for {field} in timezone {timezone}")]
    NonexistentLocalDateTime {
        field: &'static str,
        timezone: String,
    },
    #[error("custom_date_range requires custom_start_date and custom_end_date")]
    MissingCustomDateRange,
    #[error("custom_end_date must be greater than or equal to custom_start_date")]
    InvalidCustomDateRange,
    #[error("invalid period profile definition {profile_code}: {reason}")]
    InvalidProfileDefinition {
        profile_code: String,
        reason: String,
    },
}

impl ResolvedPeriodWindow {
    pub fn included_dates(&self) -> Vec<String> {
        let start_date = self.start_datetime_local.date();
        (0..self.duration_days)
            .map(|offset| {
                (start_date + Duration::days(offset))
                    .format("%Y-%m-%d")
                    .to_string()
            })
            .collect()
    }

    pub fn start_datetime_utc(&self) -> Result<String, PeriodWindowError> {
        local_datetime_to_utc_with_field(
            &self.timezone,
            self.start_datetime_local,
            "start_datetime_local",
        )
    }

    pub fn end_datetime_utc(&self) -> Result<String, PeriodWindowError> {
        local_datetime_to_utc_with_field(
            &self.timezone,
            self.end_datetime_local,
            "end_datetime_local",
        )
    }
}

impl PeriodWindowResolver {
    pub fn new(profiles: impl IntoIterator<Item = PeriodProfileDefinition>) -> Self {
        Self {
            profiles: profiles
                .into_iter()
                .map(|profile| (profile.period_profile_code.clone(), profile))
                .collect(),
        }
    }

    pub fn resolve(
        &self,
        request: &PeriodWindowRequest,
    ) -> Result<ResolvedPeriodWindow, PeriodWindowError> {
        let profile = self
            .profiles
            .get(&request.period_profile_code)
            .ok_or_else(|| {
                PeriodWindowError::UnknownProfile(request.period_profile_code.clone())
            })?;
        if !profile.is_enabled {
            return Err(PeriodWindowError::DisabledProfile(
                profile.period_profile_code.clone(),
            ));
        }
        validate_included_days(profile)?;

        request
            .timezone
            .parse::<Tz>()
            .map_err(|_| PeriodWindowError::InvalidTimezone(request.timezone.clone()))?;

        let anchor_date = parse_date("anchor_date", &request.anchor_date)?;
        let (start_date, end_date) = match profile.resolution_strategy.as_str() {
            "anchor_day" => (anchor_date, anchor_date + Duration::days(1)),
            "anchor_forward_days" => {
                let duration = required_duration(profile)?;
                (anchor_date, anchor_date + Duration::days(duration))
            }
            "iso_week" => {
                let duration = required_duration(profile)?;
                let week_offset = profile.week_offset.unwrap_or(0);
                let start = iso_week_monday(anchor_date) + Duration::days(week_offset * 7);
                (start, start + Duration::days(duration))
            }
            "iso_workweek" => {
                let duration = required_duration(profile)?;
                let week_offset = profile.week_offset.unwrap_or(0);
                let start = iso_week_monday(anchor_date) + Duration::days(week_offset * 7);
                (start, start + Duration::days(duration))
            }
            "custom_date_range" => {
                let start = request
                    .custom_start_date
                    .as_deref()
                    .ok_or(PeriodWindowError::MissingCustomDateRange)
                    .and_then(|value| parse_date("custom_start_date", value))?;
                let end = request
                    .custom_end_date
                    .as_deref()
                    .ok_or(PeriodWindowError::MissingCustomDateRange)
                    .and_then(|value| parse_date("custom_end_date", value))?;
                if end < start {
                    return Err(PeriodWindowError::InvalidCustomDateRange);
                }
                (start, end + Duration::days(1))
            }
            other => {
                return Err(PeriodWindowError::InvalidProfileDefinition {
                    profile_code: profile.period_profile_code.clone(),
                    reason: format!("unsupported resolution_strategy {other}"),
                })
            }
        };

        let duration_days = (end_date - start_date).num_days();
        Ok(ResolvedPeriodWindow {
            start_datetime_local: start_date
                .and_hms_opt(0, 0, 0)
                .expect("midnight should be valid"),
            end_datetime_local: end_date
                .and_hms_opt(0, 0, 0)
                .expect("midnight should be valid"),
            timezone: request.timezone.clone(),
            duration_days,
            end_exclusive: true,
            included_days: profile.included_days.clone(),
        })
    }
}

fn required_duration(profile: &PeriodProfileDefinition) -> Result<i64, PeriodWindowError> {
    match profile.duration_days {
        Some(value) if value > 0 => Ok(value),
        _ => Err(PeriodWindowError::InvalidProfileDefinition {
            profile_code: profile.period_profile_code.clone(),
            reason: "duration_days must be a positive integer".into(),
        }),
    }
}

fn validate_included_days(profile: &PeriodProfileDefinition) -> Result<(), PeriodWindowError> {
    let mut seen = Vec::<&str>::new();
    for day in &profile.included_days {
        let value = day.as_str();
        if !matches!(
            value,
            "monday" | "tuesday" | "wednesday" | "thursday" | "friday" | "saturday" | "sunday"
        ) {
            return Err(PeriodWindowError::InvalidProfileDefinition {
                profile_code: profile.period_profile_code.clone(),
                reason: format!("invalid included day {day}"),
            });
        }
        if seen.contains(&value) {
            return Err(PeriodWindowError::InvalidProfileDefinition {
                profile_code: profile.period_profile_code.clone(),
                reason: format!("duplicate included day {day}"),
            });
        }
        seen.push(value);
    }
    Ok(())
}

fn parse_date(field: &'static str, value: &str) -> Result<NaiveDate, PeriodWindowError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| PeriodWindowError::InvalidDate {
        field,
        value: value.to_string(),
    })
}

fn local_datetime_to_utc_with_field(
    timezone: &str,
    local: NaiveDateTime,
    field: &'static str,
) -> Result<String, PeriodWindowError> {
    let tz = timezone
        .parse::<Tz>()
        .map_err(|_| PeriodWindowError::InvalidTimezone(timezone.to_string()))?;
    match tz.from_local_datetime(&local) {
        chrono::LocalResult::Single(value) => Ok(value.with_timezone(&chrono::Utc).to_rfc3339()),
        chrono::LocalResult::Ambiguous(_, _) => Err(PeriodWindowError::AmbiguousLocalDateTime {
            field,
            timezone: timezone.to_string(),
        }),
        chrono::LocalResult::None => Err(PeriodWindowError::NonexistentLocalDateTime {
            field,
            timezone: timezone.to_string(),
        }),
    }
}

fn iso_week_monday(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

fn default_enabled() -> bool {
    true
}
