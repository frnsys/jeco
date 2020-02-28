use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub struct SimulationConfig {
    pub population: usize,
    pub n_publishers: usize,
    pub n_platforms: usize,

    // Base offline contact rate
    pub contact_rate: f32,

    // Horizontal stretching of gravity function,
    // higher values mean weaker influence at greater distances
    pub gravity_stretch: f32,

    // Attention budget per Agent
    pub attention_budget: f32,

    // Maximum movement amount
    pub max_influence: f32,

    // How much content a Publisher
    // surveys to understand their audience
    pub content_sample_size: usize,

    // Base probability of signing up
    // to a social network
    pub base_signup_rate: f32,

    // How much data is generated
    // per content consumption
    pub data_per_consume: f32,

    // Max number of Platforms
    // an Agent signs up for
    pub max_platforms: usize,

    // Revenue generated per ad view
    pub revenue_per_ad: f32,

    // How quickly Publishers and Agents adjust to
    // learnings re ads/quality
    pub change_rate: f32,

    // If trust goes below/above this threshold,
    // unfollow/follow that Agent
    pub unfollow_trust: f32,
    pub follow_trust: f32,

    // If trust goes below/above this threshold,
    // unsubscribe/subscribe from that Publisher
    pub unsubscribe_trust: f32,
    pub subscribe_trust: f32,

    // Agents unsubscribe from Publishers
    // if they don't see Content from them for
    // this many steps
    pub unsubscribe_lag: usize,

    // Base conversion rate for ads
    pub base_conversion_rate: f32,

    // Conversion rate limit for ads
    pub max_conversion_rate: f32,

    // Maximum amount of shared content
    // an agent considers. Setting this
    // too high can severely slow things down.
    pub max_shared_content: usize,

    // See below
    pub publisher: PublisherConfig,
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
