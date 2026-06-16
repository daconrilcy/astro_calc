#[cfg(feature = "swisseph-engine")]
#[test]
fn swiss_ephemeris_smoke() -> Result<(), Box<dyn std::error::Error>> {
    use swiss_eph::safe::{calc_ut, close, julday, CalcFlags, Planet};

    let jd_ut = julday(2024, 6, 15, 12.0);
    let flags = CalcFlags::new().with_moshier().with_speed();

    let sun = calc_ut(jd_ut, Planet::Sun.to_int(), flags.raw())?;

    assert!(jd_ut.is_finite());
    assert!(sun.longitude.is_finite());
    assert!(sun.latitude.is_finite());
    assert!(sun.longitude_speed.is_finite());

    close();

    Ok(())
}

#[cfg(not(feature = "swisseph-engine"))]
#[test]
fn swiss_ephemeris_smoke_feature_disabled() {
    assert!(true);
}
