use super::motive::Motive;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub struct SimulationConfig {
    pub grid_size: usize,
    pub population: usize,
    pub n_publishers: usize,
    pub n_platforms: usize,

    // Base offline contact rate
    pub contact_rate: f32,

    // Horizontal stretching of gravity function,
    // higher values mean weaker influence at greater distances
    pub gravity_stretch: f32,

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

    // Initialize trust to this value
    pub default_trust: f32,

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

    // How much it costs for 1 point of quality
    pub cost_per_quality: f32,

    // General strength of the economy
    pub economy: f32,

    // See below
    pub publisher: PublisherConfig,
    pub agent: AgentConfig,

    pub publishers: Vec<SinglePublisherConfig>,
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

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub struct SinglePublisherConfig {
    // Base budget for Publisher
    pub base_budget: f32,
    pub motive: Motive
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub struct AgentConfig {
    // Attention budget per Agent
    pub attention_budget: f32,
}

impl SimulationConfig {
    pub fn default() -> SimulationConfig {
        SimulationConfig {
            grid_size: 3,
            population: 1000,
            n_publishers: 10,
            n_platforms: 10,
            contact_rate: 0.1,
            gravity_stretch: 10.,
            max_influence: 0.1,
            content_sample_size: 50,
            base_signup_rate: 0.001,
            data_per_consume: 0.0001,
            max_platforms: 3,
            revenue_per_ad: 0.001,
            default_trust: 0.4,
            unfollow_trust: 0.1,
            follow_trust: 0.9,
            unsubscribe_trust: 0.1,
            subscribe_trust: 0.9,
            unsubscribe_lag: 20,
            base_conversion_rate: 0.01,
            economy: 1.,
            max_conversion_rate: 0.05,
            max_shared_content: 200,
            cost_per_quality: 0.5,
            publisher: PublisherConfig {
                revenue_per_subscriber: 0.01,
                base_budget: 2000.
            },
            agent: AgentConfig {
                attention_budget: 20.
            },
            publishers: Vec::new()
        }
    }
}
