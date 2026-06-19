#![allow(deprecated)]

use astral_calculator::aspects::canonical_aspect_orb_deg;
use astral_calculator::catalog::test_catalog;
use astral_calculator::cli::OutputContract;
use astral_calculator::config::runtime_options_from_env;
use astral_calculator::db::connect_from_env;
use astral_calculator::dignities::essential_dignities_for_positions;
use astral_calculator::ephemeris::SwissEphemerisEngine;
use astral_calculator::facts::shortest_angular_distance;
use astral_calculator::idempotency::advisory_lock_key;

#[test]
fn deprecated_root_aliases_still_compile_for_public_compatibility() {
    let _ = canonical_aspect_orb_deg;
    let _ = test_catalog();
    let _ = std::mem::discriminant(&OutputContract::Engine);
    let _ = runtime_options_from_env;
    let _ = connect_from_env;
    let _ = essential_dignities_for_positions;
    let _ = std::any::type_name::<SwissEphemerisEngine>();
    let _ = shortest_angular_distance(10.0, 350.0);
    let _ = advisory_lock_key("compat");
}
