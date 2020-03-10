mod sim;
mod util;
mod grid;
mod agent;
mod policy;
mod content;
mod network;
mod platform;
mod publisher;
mod config;

pub use self::policy::Policy;
pub use self::sim::Simulation;
pub use self::agent::{Agent, Values, AgentId};
pub use self::config::SimulationConfig;
pub use self::publisher::{Publisher, PublisherId};
pub use self::grid::Position;


#[cfg(test)]
mod tests {
    use super::*;
    use super::grid::HexGrid;
    use super::agent::Topics;
    use super::platform::PlatformId;
    use super::config::{AgentConfig, PublisherConfig};
    use super::content::{Content, ContentId, ContentBody, SharedContent, SharerType};
    use super::sim::set_agent_relevancies;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use rand::seq::SliceRandom;
    use std::rc::Rc;

    fn standard_agents(conf: &SimulationConfig, rng: &mut StdRng) -> Vec<Agent> {
        (0..100).map(|i| {
            let mut agent = Agent::new(i, &conf.agent, rng);
            agent.values.set(Values::from_vec(vec![0., 0.]));
            agent.interests = Topics::from_vec(vec![1., 1.]);
            agent.attention = 100.;
            agent.location = (0, 0);
            agent
        }).collect()
    }

    // Agents-only
    #[test]
    fn test_influence() {
        let trust = 1.;
        let gravity_stretch = 100.;
        let max_influence = 0.1;
        let conf = AgentConfig {
            attention_budget: 100.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut consumer = Agent::new(0, &conf, &mut rng);
        consumer.values.set(Values::from_vec(vec![0., 0.]));
        let producer = Agent::new(1, &conf, &mut rng);
        producer.values.set(Values::from_vec(vec![-1., -1.]));

        for _ in 0..2000 {
            let body = producer.produce(conf.attention_budget, &mut rng);
            consumer.be_influenced(&body.values, gravity_stretch, max_influence, trust);
        }

        // Should both be close to -1.
        let x = consumer.values.get()[0];
        let y = consumer.values.get()[1];
        assert!(x - -1. < 0.05);
        assert!(y - -1. < 0.05);

        producer.values.set(Values::from_vec(vec![1., 1.]));
        for _ in 0..3000 {
            let body = producer.produce(conf.attention_budget, &mut rng);
            consumer.be_influenced(&body.values, gravity_stretch, max_influence, trust);
        }

        // Should both be close to 1.
        let x = consumer.values.get()[0];
        let y = consumer.values.get()[1];
        assert!(x - 1. < 0.05);
        assert!(y - 1. < 0.05);
    }

    #[test]
    fn test_polarization() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };
        conf.gravity_stretch = 100.;
        conf.max_influence = 0.1;

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let consumers: Vec<Agent> = (0..100).map(|i| Agent::new(i, &conf.agent, &mut rng)).collect();
        for consumer in &consumers {
            consumer.values.set(Values::from_vec(vec![0., 0.]));
        }

        let mut producers: Vec<Agent> = Vec::new();
        let values = vec![
            vec![-1., -1.],
            vec![-1.,  1.],
            vec![ 1.,  1.],
            vec![ 1., -1.],
        ];
        for i in 0..10 {
            for j in 0..4 {
                let id =  consumers.len() + (i * 4) + j;
                let producer = Agent::new(id, &conf.agent, &mut rng);
                producer.values.set(Values::from_vec(values[j].clone()));
                producers.push(producer);
            }
        }

        for _ in 0..100 {
            let content: Vec<SharedContent> = producers.iter().map(|p| {
                // Self-published and no ads only
                let mut body = p.produce(conf.agent.attention_budget, &mut rng);

                // Control for cost & quality
                // Topic allowed to vary
                body.cost = 10.;
                body.quality = 1.;

                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: None,
                    author: p.id,
                    body: body,
                    ads: 0.
                };
                SharedContent {
                    content: Rc::new(content),
                    sharer: (SharerType::Agent, p.id)
                }
            }).collect();

            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = content.iter()
                .map(|c| (None, c)).collect();
            for a in &consumers {
                shared.shuffle(&mut rng);
                a.consume(&shared, &conf, &mut rng);
            }
        }

        // TODO
        // for a in &consumers {
        //     println!("{:?}", a.values);
        // }
    }

    #[test]
    fn test_high_quality_shared_more() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let consumers = standard_agents(&conf, &mut rng);

        let low = 0.1;
        let high = 1.0;
        let mut high_quality_shares = 0;
        let mut low_quality_shares = 0;

        let author_id = consumers.len();
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: None,
                    author: author_id,
                    body: ContentBody {
                        // High quality vs low quality
                        quality: if i < 50 {low} else {high},
                        topics: Topics::from_vec(vec![1., 1.]),
                        values: Values::from_vec(vec![0., 0.]),
                        cost: 10.,
                    },
                    ads: 0.
                };
                SharedContent {
                    content: Rc::new(content),
                    sharer: (SharerType::Agent, author_id)
                }
            }).collect();

            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = content.iter()
                .map(|c| (None, c)).collect();
            for a in &consumers {
                shared.shuffle(&mut rng);
                let (will_share, _, _, _, _) = a.consume(&shared, &conf, &mut rng);
                for shared in will_share {
                    if shared.body.quality == high {
                        high_quality_shares += 1;
                    } else {
                        low_quality_shares += 1;
                    }
                }
            }
        }
        println!("high:{:?} low:{:?}", high_quality_shares, low_quality_shares);
        assert!(high_quality_shares > low_quality_shares);
    }

    #[test]
    fn test_low_attention_shared_more() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let consumers = standard_agents(&conf, &mut rng);

        let low = 0.1;
        let high = 10.0;
        let mut high_attention_shares = 0;
        let mut low_attention_shares = 0;

        let author_id = consumers.len();
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: None,
                    author: author_id,
                    body: ContentBody {
                        // High attention vs low attention
                        cost: if i < 50 {low} else {high},
                        topics: Topics::from_vec(vec![1., 1.]),
                        values: Values::from_vec(vec![0., 0.]),
                        quality: 1.,
                    },
                    ads: 0.
                };
                SharedContent {
                    content: Rc::new(content),
                    sharer: (SharerType::Agent, author_id)
                }
            }).collect();

            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = content.iter()
                .map(|c| (None, c)).collect();
            for a in &consumers {
                shared.shuffle(&mut rng);
                let (will_share, _, _, _, _) = a.consume(&shared, &conf, &mut rng);
                for shared in will_share {
                    if shared.body.cost == high {
                        high_attention_shares += 1;
                    } else {
                        low_attention_shares += 1;
                    }
                }
            }
        }
        println!("high:{:?} low:{:?}", high_attention_shares, low_attention_shares);
        assert!(low_attention_shares > high_attention_shares);
    }

    #[test]
    fn test_more_aligned_shared_more() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let consumers = standard_agents(&conf, &mut rng);

        let aligned = Values::from_vec(vec![0., 0.]);
        let not_aligned = Values::from_vec(vec![-1., -1.]);
        let mut aligned_shares = 0;
        let mut not_aligned_shares = 0;

        let author_id = consumers.len();
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: None,
                    author: author_id,
                    body: ContentBody {
                        topics: Topics::from_vec(vec![1., 1.]),
                        values: if i < 50 {aligned.clone()} else {not_aligned.clone()},
                        cost: 10.,
                        quality: 1.,
                    },
                    ads: 0.
                };
                SharedContent {
                    content: Rc::new(content),
                    sharer: (SharerType::Agent, author_id)
                }
            }).collect();

            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = content.iter()
                .map(|c| (None, c)).collect();
            for a in &consumers {
                shared.shuffle(&mut rng);
                let (will_share, _, _, _, _) = a.consume(&shared, &conf, &mut rng);
                for shared in will_share {
                    if shared.body.values == aligned {
                        aligned_shares += 1;
                    } else {
                        not_aligned_shares += 1;
                    }
                }
            }
        }
        println!("aligned:{:?} not:{:?}", aligned_shares, not_aligned_shares);
        assert!(aligned_shares > not_aligned_shares);
    }

    #[test]
    fn test_more_affinity_shared_more() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let consumers = standard_agents(&conf, &mut rng);

        let aligned = Topics::from_vec(vec![1., 1.]);
        let not_aligned = Topics::from_vec(vec![0., 0.]);
        let mut aligned_shares = 0;
        let mut not_aligned_shares = 0;

        let author_id = consumers.len();
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: None,
                    author: author_id,
                    body: ContentBody {
                        topics: if i < 50 {aligned.clone()} else {not_aligned.clone()},
                        values: Values::from_vec(vec![0., 0.]),
                        cost: 10.,
                        quality: 1.,
                    },
                    ads: 0.
                };
                SharedContent {
                    content: Rc::new(content),
                    sharer: (SharerType::Agent, author_id)
                }
            }).collect();

            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = content.iter()
                .map(|c| (None, c)).collect();
            for a in &consumers {
                shared.shuffle(&mut rng);
                let (will_share, _, _, _, _) = a.consume(&shared, &conf, &mut rng);
                for shared in will_share {
                    if shared.body.topics == aligned {
                        aligned_shares += 1;
                    } else {
                        not_aligned_shares += 1;
                    }
                }
            }
        }
        println!("affinity:{:?} not:{:?}", aligned_shares, not_aligned_shares);
        assert!(aligned_shares > not_aligned_shares);
    }

    #[test]
    fn test_more_relevant_shared_more() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };
        conf.publisher = PublisherConfig {
            revenue_per_subscriber: 10.,
            base_budget: 10000.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut consumers = standard_agents(&conf, &mut rng);

        let near_id = 0;
        let mut publisher_near = Publisher::new(near_id, &conf.publisher, &mut rng);
        publisher_near.location = (0, 0);
        publisher_near.radius = 1;

        let medium_id = 1;
        let mut publisher_medium = Publisher::new(medium_id, &conf.publisher, &mut rng);
        publisher_medium.location = (2, 2);
        publisher_medium.radius = 2;

        let far_id = 2;
        let mut publisher_far = Publisher::new(far_id, &conf.publisher, &mut rng);
        publisher_far.location = (6, 6);
        publisher_far.radius = 1;

        let grid = HexGrid::new(8, 8);
        let publishers = vec![publisher_near, publisher_medium, publisher_far];
        set_agent_relevancies(&grid, &mut consumers, &publishers);

        let mut near_shares = 0;
        let mut medium_shares = 0;
        let mut far_shares = 0;


        let author_id = consumers.len();
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..120).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(if i < 40 {
                        near_id
                    } else if i < 80 {
                        medium_id
                    } else {
                        far_id
                    }),
                    author: author_id,
                    body: ContentBody {
                        topics: Topics::from_vec(vec![1., 1.]),
                        values: Values::from_vec(vec![0., 0.]),
                        cost: 10.,
                        quality: 1.,
                    },
                    ads: 0.
                };
                SharedContent {
                    content: Rc::new(content),
                    sharer: (SharerType::Agent, author_id)
                }
            }).collect();

            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = content.iter()
                .map(|c| (None, c)).collect();
            for a in &consumers {
                shared.shuffle(&mut rng);
                let (will_share, _, _, _, _) = a.consume(&shared, &conf, &mut rng);
                for shared in will_share {
                    match shared.publisher {
                        Some(publisher) => {
                            if publisher == near_id {
                                near_shares += 1;
                            } else if publisher == medium_id {
                                medium_shares += 1;
                            } else {
                                far_shares += 1;
                            }
                        },
                        None => {}
                    }
                }
            }
        }
        println!("near:{:?} medium:{:?} far:{:?}", near_shares, medium_shares, far_shares);
        assert!(near_shares > medium_shares);
        assert!(medium_shares > far_shares);
    }

    #[test]
    fn test_rich_produce_more() {
        let mut conf = SimulationConfig::default();
        conf.cost_per_quality = 0.2;
        conf.agent = AgentConfig {
            attention_budget: 100.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut rich: Vec<Agent> = (0..100).map(|i| {
            let mut agent = Agent::new(i, &conf.agent, &mut rng);
            agent.resources = 100.;

            // Control for quality and reach
            agent.quality = 10.;
            agent.reach = 1.;

            agent
        }).collect();
        let mut poor: Vec<Agent> = (0..100).map(|i| {
            let mut agent = Agent::new(rich.len() + i, &conf.agent, &mut rng);
            agent.resources = 5.;

            // Control for quality
            agent.quality = 10.;
            agent.reach = 1.;

            agent
        }).collect();

        let mut rich_content = 0;
        let mut poor_content = 0;
        for _ in 0..100 {
            for a in &mut rich {
                match a.try_produce(&conf, &mut rng) {
                    Some(_) => rich_content += 1,
                    None => {}
                }
            }

            for a in &mut poor {
                match a.try_produce(&conf, &mut rng) {
                    Some(_) => poor_content += 1,
                    None => {}
                }
            }
        }

        println!("rich:{:?} poor:{:?}", rich_content, poor_content);
        assert!(rich_content > poor_content);
    }
}
