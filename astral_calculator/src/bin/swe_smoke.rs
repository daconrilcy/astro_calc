use swiss_eph::safe::{calc_ut, close, julday, CalcFlags, Planet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let jd_ut = julday(2024, 6, 15, 12.0);
    let flags = CalcFlags::new().with_moshier().with_speed();

    let sun = calc_ut(jd_ut, Planet::Sun.to_int(), flags.raw())?;

    println!("JD UT: {}", jd_ut);
    println!("Soleil longitude: {:.6} deg", sun.longitude);
    println!("Soleil latitude: {:.6} deg", sun.latitude);
    println!("Vitesse longitude: {:.6} deg/jour", sun.longitude_speed);

    close();

    Ok(())
}
