#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BasicAccidentalScoringSnapshot {
    pub overall_score_baseline: f64,
    pub overall_score_min: f64,
    pub overall_score_max: f64,
    pub angle_proximity_max_orb_deg: f64,
    pub polarity_bands: Vec<AccidentalPolarityBand>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BasicProductScoringSnapshot {
    pub sign_house_emphasis_min_score: f64,
    pub object_emphasis_min_score: f64,
    pub max_dominant_signs: usize,
    pub max_dominant_houses: usize,
    pub max_dominant_objects: usize,
    pub max_active_signals: usize,
    pub aspect_min_strength: f64,
    pub max_house_axis_emphasis: usize,
}

use serde::{Deserialize, Serialize};

use super::AccidentalPolarityBand;
