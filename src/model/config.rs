use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub struct SimulationConfig {
    pub population: usize,
    pub n_publishers: usize,

    // Horizontal stretching of gravity function,
    // higher values mean weaker influence at greater distances
    pub gravity_stretch: f32,

    // Maximum movement amount
    pub max_influence: f32,

    // Multiplier for subscription probabilities
    // to dampen subscription rates
    pub subscription_prob_weight: f32,

    pub publisher: PublisherConfig
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub struct PublisherConfig {
    // How much each subscriber adds
    // to Publishers' budgets
    pub revenue_per_subscriber: f32,

    // Base budget for Publishers
    pub base_budget: f32,
}