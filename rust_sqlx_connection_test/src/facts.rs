use crate::domain::HouseCuspFact;

pub fn normalize_degrees(value: f64) -> f64 {
    let normalized = value % 360.0;
    if normalized < 0.0 {
        normalized + 360.0
    } else {
        normalized
    }
}

pub fn sign_id_for_longitude(longitude_deg: f64) -> i32 {
    (normalize_degrees(longitude_deg) / 30.0).floor() as i32 + 1
}

pub fn motion_state_id(speed_deg_per_day: Option<f64>) -> Option<i32> {
    let speed = speed_deg_per_day?;
    if speed.abs() <= 0.0001 {
        Some(3)
    } else if speed < 0.0 {
        Some(2)
    } else {
        Some(1)
    }
}

pub fn whole_sign_house_id(ascendant_longitude_deg: f64, body_longitude_deg: f64) -> i32 {
    let asc_sign = sign_id_for_longitude(ascendant_longitude_deg);
    let body_sign = sign_id_for_longitude(body_longitude_deg);
    ((body_sign - asc_sign).rem_euclid(12)) + 1
}

pub fn house_id_from_cusps(longitude_deg: f64, cusps: &[HouseCuspFact]) -> Option<i32> {
    if cusps.len() != 12 {
        return None;
    }

    let longitude = normalize_degrees(longitude_deg);
    for index in 0..12 {
        let start = normalize_degrees(cusps[index].longitude_deg);
        let end = normalize_degrees(cusps[(index + 1) % 12].longitude_deg);
        if arc_contains(start, end, longitude) {
            return Some(cusps[index].house_id);
        }
    }

    None
}

pub fn arc_contains(start: f64, end: f64, value: f64) -> bool {
    if start <= end {
        value >= start && value < end
    } else {
        value >= start || value < end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_handles_wraparound() {
        assert_eq!(sign_id_for_longitude(0.0), 1);
        assert_eq!(sign_id_for_longitude(359.9), 12);
        assert_eq!(sign_id_for_longitude(-1.0), 12);
    }

    #[test]
    fn arc_contains_handles_zero_crossing() {
        assert!(arc_contains(350.0, 10.0, 2.0));
        assert!(arc_contains(350.0, 10.0, 355.0));
        assert!(!arc_contains(350.0, 10.0, 180.0));
    }
}
