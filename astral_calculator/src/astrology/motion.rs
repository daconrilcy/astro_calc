//! Résolution métier des états de mouvement apparent.

use crate::domain::MotionStateReference;

/// Retourne l'état de mouvement correspondant à la vitesse apparente.
pub fn motion_state_for_speed(
    speed_deg_per_day: Option<f64>,
    motion_states: &[MotionStateReference],
) -> Option<&MotionStateReference> {
    let speed = speed_deg_per_day?;
    let code = if speed.abs() <= 0.0001 {
        "stationary"
    } else if speed < 0.0 {
        "retrograde"
    } else {
        "direct"
    };
    motion_states.iter().find(|state| state.code == code)
}
