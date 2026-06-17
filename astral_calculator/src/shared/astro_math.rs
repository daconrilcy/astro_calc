use crate::domain::HouseCuspFact;

pub fn normalize_degrees(value: f64) -> f64 {
    let normalized = value % 360.0;
    if normalized < 0.0 {
        normalized + 360.0
    } else {
        normalized
    }
}

pub fn shortest_angular_distance(left: f64, right: f64) -> f64 {
    let diff = (normalize_degrees(left) - normalize_degrees(right)).abs();
    diff.min(360.0 - diff)
}

pub fn zodiac_slot_for_longitude(longitude_deg: f64) -> i32 {
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

pub fn whole_sign_house_number(ascendant_longitude_deg: f64, body_longitude_deg: f64) -> i32 {
    let asc_sign = zodiac_slot_for_longitude(ascendant_longitude_deg);
    let body_sign = zodiac_slot_for_longitude(body_longitude_deg);
    ((body_sign - asc_sign).rem_euclid(12)) + 1
}

pub fn house_number_from_cusps(longitude_deg: f64, cusps: &[HouseCuspFact]) -> Option<i32> {
    if cusps.len() != 12 {
        return None;
    }

    let longitude = normalize_degrees(longitude_deg);
    for index in 0..12 {
        let start = normalize_degrees(cusps[index].longitude_deg);
        let end = normalize_degrees(cusps[(index + 1) % 12].longitude_deg);
        if arc_contains(start, end, longitude) {
            return Some(cusps[index].house_number);
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
