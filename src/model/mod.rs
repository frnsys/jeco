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
    use super::platform::{Platform, PlatformId};
    use super::config::{AgentConfig, PublisherConfig};
    use super::content::{Content, ContentId, ContentBody, SharedContent, SharerType};
    use self::publisher::Audience;
    use super::sim::{set_agent_relevancies, ad_market};
    use super::util::Vector;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use rand::seq::SliceRandom;
    use std::rc::Rc;
    use fnv::FnvHashMap;

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
    fn influence() {
        let trust = 1.;
        let gravity_stretch = 10.;
        let max_influence = 0.1;
        let conf = AgentConfig {
            attention_budget: 100.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let consumer = Agent::new(0, &conf, &mut rng);
        consumer.values.set(Values::from_vec(vec![0., 0.]));
        let producer = Agent::new(1, &conf, &mut rng);
        producer.values.set(Values::from_vec(vec![-1., -1.]));

        for _ in 0..200 {
            let body = producer.produce(conf.attention_budget, &mut rng);
            consumer.be_influenced(&body.values, gravity_stretch, max_influence, trust);
        }

        // Should both be close to -1.
        let x = consumer.values.get()[0];
        let y = consumer.values.get()[1];
        println!("x: {:?} ({:?})", x, x - -1.);
        println!("y: {:?} ({:?})", y, y - -1.);
        assert!(x - -1. < 0.1);
        assert!(y - -1. < 0.1);

        producer.values.set(Values::from_vec(vec![1., 1.]));
        for _ in 0..500 {
            let body = producer.produce(conf.attention_budget, &mut rng);
            consumer.be_influenced(&body.values, gravity_stretch, max_influence, trust);
        }

        // Should both be close to 1.
        let x = consumer.values.get()[0];
        let y = consumer.values.get()[1];
        println!("x: {:?} ({:?})", x, (x - 1.).abs());
        println!("y: {:?} ({:?})", y, (y - 1.).abs());
        assert!((x - 1.).abs() < 0.1);
        assert!((y - 1.).abs() < 0.1);
    }

    #[test]
    fn producers_produce_aligned_content() {
        let conf = SimulationConfig::default();
        let mut rng: StdRng = SeedableRng::seed_from_u64(0);

        let trials = 100;
        let max_distance = 0.1;
        let values = vec![
            vec![-1., -1.],
            vec![-1.,  1.],
            vec![ 1.,  1.],
            vec![ 1., -1.],
        ];
        for v in values {
            let producer = Agent::new(0, &conf.agent, &mut rng);
            let mut count = 0;
            producer.values.set(Values::from_vec(v));
            for _ in 0..trials {
                let body = producer.produce(conf.agent.attention_budget, &mut rng);
                let values = body.values;
                let p_vals = producer.values.get();
                if (p_vals[0] - values[0]).abs() <= max_distance && (p_vals[1] - values[1]).abs() <= max_distance {
                    count += 1;
                }
            }
            let p_acceptable = count as f32/trials as f32;
            // println!("{:?}", p_acceptable);
            assert!(p_acceptable >= 0.90);
        }
    }

    #[test]
    fn polarization() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };
        conf.gravity_stretch = 10.;
        conf.max_influence = 1.0;

        // Create centrist agents
        let center_values = vec![
            vec![-0.25, -0.25],
            vec![-0.25,  0.25],
            vec![ 0.25,  0.25],
            vec![ 0.25, -0.25],
        ];
        let mut rng: StdRng = SeedableRng::seed_from_u64(1);
        let consumers: Vec<Agent> = (0..4).map(|i| {
            let agent = Agent::new(i, &conf.agent, &mut rng);
            agent.values.set(Values::from_vec(center_values[i].clone()));
            agent
        }).collect();

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

        for _ in 0..300 {
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

        // Assert agent values are now
        // within max_distance of the extremes
        let max_distance = 0.15;
        for i in 0..4 {
            let agent = &consumers[i];
            let a_vals = agent.values.get();
            let values = &values[i];
            // println!("{:?}", a_vals);
            assert!((a_vals[0] - values[0]).abs() <= max_distance && (a_vals[1] - values[1]).abs() <= max_distance);
        }
    }

    #[test]
    fn high_quality_shared_more() {
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
        for _ in 0..10 {
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
    fn low_attention_shared_more() {
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
        for _ in 0..10 {
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
    fn more_aligned_shared_more() {
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
        for _ in 0..10 {
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
    fn more_affinity_shared_more() {
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
        for _ in 0..10 {
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
    fn more_relevant_shared_more() {
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
        for _ in 0..10 {
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
    fn rich_produce_more() {
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

    #[test]
    fn alignment_changes_trust() {
        // Tests both trust of sharer and author
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);

        let agent_a = Agent::new(0, &conf.agent, &mut rng);
        agent_a.values.set(Values::from_vec(vec![-1., -1.]));

        let agent_b = Agent::new(1, &conf.agent, &mut rng);
        agent_b.values.set(Values::from_vec(vec![ 1.,  1.]));

        let producer_a = 2;
        let producer_b = 3;
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let author_id = if i < 50 {producer_a} else {producer_b};
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: None,
                    author: author_id,
                    body: ContentBody {
                        topics: Topics::from_vec(vec![1., 1.]),
                        values: if i < 50 {
                            Values::from_vec(vec![-1., -1.])
                        } else {
                            Values::from_vec(vec![ 1.,  1.])
                        },
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
            shared.shuffle(&mut rng);
            agent_a.consume(&shared, &conf, &mut rng);
            shared.shuffle(&mut rng);
            agent_b.consume(&shared, &conf, &mut rng);
        }

        let trust_a = agent_a.trust.borrow();
        println!("a trust of p_a:{:?} p_b:{:?}", trust_a.get(&producer_a).unwrap(), trust_a.get(&producer_b).unwrap());
        assert!(*trust_a.get(&producer_a).unwrap() > 0.8);
        assert_eq!(*trust_a.get(&producer_b).unwrap(), 0.0);

        let trust_b = agent_b.trust.borrow();
        println!("b trust of p_a:{:?} p_b:{:?}", trust_b.get(&producer_a).unwrap(), trust_b.get(&producer_b).unwrap());
        assert_eq!(*trust_b.get(&producer_a).unwrap(), 0.0);
        assert!(*trust_b.get(&producer_b).unwrap() > 0.8);
    }

    #[test]
    fn affinity_changes_trust() {
        // Tests both trust of sharer and author
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);

        let mut agent_a = Agent::new(0, &conf.agent, &mut rng);
        agent_a.interests = Topics::from_vec(vec![ 0., 1.]);

        let mut agent_b = Agent::new(1, &conf.agent, &mut rng);
        agent_b.interests = Topics::from_vec(vec![ 1., 0.]);

        let producer_a = 2;
        let producer_b = 3;
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let author_id = if i < 50 {producer_a} else {producer_b};
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: None,
                    author: author_id,
                    body: ContentBody {
                        topics: if i < 50 {
                            Topics::from_vec(vec![ 0., 1.])
                        } else {
                            Topics::from_vec(vec![ 1., 0.])
                        },
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
            shared.shuffle(&mut rng);
            agent_a.consume(&shared, &conf, &mut rng);
            shared.shuffle(&mut rng);
            agent_b.consume(&shared, &conf, &mut rng);
        }

        let trust_a = agent_a.trust.borrow();
        println!("a trust of p_a:{:?} p_b:{:?}", trust_a.get(&producer_a).unwrap(), trust_a.get(&producer_b).unwrap());
        assert!(*trust_a.get(&producer_a).unwrap() > 0.8);
        assert_eq!(*trust_a.get(&producer_b).unwrap(), 0.0);

        let trust_b = agent_b.trust.borrow();
        println!("b trust of p_a:{:?} p_b:{:?}", trust_b.get(&producer_a).unwrap(), trust_b.get(&producer_b).unwrap());
        assert_eq!(*trust_b.get(&producer_a).unwrap(), 0.0);
        assert!(*trust_b.get(&producer_b).unwrap() > 0.8);
    }

    #[test]
    fn alignment_changes_publisher_trust() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };
        conf.publisher = PublisherConfig {
            revenue_per_subscriber: 10.,
            base_budget: 10000.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut agent = Agent::new(0, &conf.agent, &mut rng);
        agent.values.set(Values::from_vec(vec![0., 0.]));
        agent.interests = Topics::from_vec(vec![1., 1.]);
        agent.location = (0, 0);
        agent.relevancies.push(1.0); // Publisher a
        agent.relevancies.push(1.0); // Publisher b

        // Control for location
        let pub_a_id = 0;
        let mut publisher_a = Publisher::new(pub_a_id, &conf.publisher, &mut rng);
        publisher_a.location = (0, 0);
        publisher_a.radius = 1;

        let pub_b_id = 1;
        let mut publisher_b = Publisher::new(pub_b_id, &conf.publisher, &mut rng);
        publisher_b.location = (0, 0);
        publisher_b.radius = 1;

        let author_id = 1;
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(if i < 50 {
                        pub_a_id
                    } else {
                        pub_b_id
                    }),
                    author: author_id,
                    body: ContentBody {
                        topics: Topics::from_vec(vec![1., 1.]),
                        values: if i < 50 {
                            Values::from_vec(vec![0., 0.])
                        } else {
                            Values::from_vec(vec![-1., -1.])
                        },
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
            shared.shuffle(&mut rng);
            agent.consume(&shared, &conf, &mut rng);
        }

        let trust = agent.publishers.borrow();
        let (pub_a_trust, _) = trust.get(&pub_a_id).unwrap();
        let (pub_b_trust, _) = trust.get(&pub_b_id).unwrap();
        println!("trust of p_a:{:?} p_b:{:?}", pub_a_trust, pub_b_trust);
        assert!(*pub_a_trust > 0.9);
        assert!(*pub_b_trust < 0.05);
    }

    #[test]
    fn affinity_changes_publisher_trust() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };
        conf.publisher = PublisherConfig {
            revenue_per_subscriber: 10.,
            base_budget: 10000.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut agent = Agent::new(0, &conf.agent, &mut rng);
        agent.values.set(Values::from_vec(vec![0., 0.]));
        agent.interests = Topics::from_vec(vec![1., 1.]);
        agent.location = (0, 0);
        agent.relevancies.push(1.0); // Publisher a
        agent.relevancies.push(1.0); // Publisher b

        // Control for location
        let pub_a_id = 0;
        let mut publisher_a = Publisher::new(pub_a_id, &conf.publisher, &mut rng);
        publisher_a.location = (0, 0);
        publisher_a.radius = 1;

        let pub_b_id = 1;
        let mut publisher_b = Publisher::new(pub_b_id, &conf.publisher, &mut rng);
        publisher_b.location = (0, 0);
        publisher_b.radius = 1;

        let author_id = 1;
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(if i < 50 {
                        pub_a_id
                    } else {
                        pub_b_id
                    }),
                    author: author_id,
                    body: ContentBody {
                        topics: if i < 50 {
                            Topics::from_vec(vec![1., 1.])
                        } else {
                            Topics::from_vec(vec![0., 0.])
                        },
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
            shared.shuffle(&mut rng);
            agent.consume(&shared, &conf, &mut rng);
        }

        let trust = agent.publishers.borrow();
        let (pub_a_trust, _) = trust.get(&pub_a_id).unwrap();
        let (pub_b_trust, _) = trust.get(&pub_b_id).unwrap();
        println!("trust of p_a:{:?} p_b:{:?}", pub_a_trust, pub_b_trust);
        assert_eq!(*pub_a_trust, 1.0);
        assert!(pub_b_trust - conf.default_trust < 0.05);
    }

    #[test]
    fn relevancy_changes_publisher_trust() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };
        conf.publisher = PublisherConfig {
            revenue_per_subscriber: 10.,
            base_budget: 10000.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut agent = Agent::new(0, &conf.agent, &mut rng);
        agent.values.set(Values::from_vec(vec![0., 0.]));
        agent.interests = Topics::from_vec(vec![1., 1.]);
        agent.location = (0, 0);
        agent.relevancies.push(1.0); // Publisher a
        agent.relevancies.push(0.0); // Publisher b

        // Control for location
        let pub_a_id = 0;
        let mut publisher_a = Publisher::new(pub_a_id, &conf.publisher, &mut rng);
        publisher_a.location = (0, 0);
        publisher_a.radius = 1;

        let pub_b_id = 1;
        let mut publisher_b = Publisher::new(pub_b_id, &conf.publisher, &mut rng);
        publisher_b.location = (0, 0);
        publisher_b.radius = 1;

        let author_id = 1;
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(if i < 50 {
                        pub_a_id
                    } else {
                        pub_b_id
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
            shared.shuffle(&mut rng);
            agent.consume(&shared, &conf, &mut rng);
        }

        let trust = agent.publishers.borrow();
        let (pub_a_trust, _) = trust.get(&pub_a_id).unwrap();
        let (pub_b_trust, _) = trust.get(&pub_b_id).unwrap();
        println!("trust of p_a:{:?} p_b:{:?}", pub_a_trust, pub_b_trust);
        assert_eq!(*pub_a_trust, 1.0);
        assert!(pub_b_trust - conf.default_trust < 0.05);
    }

    #[test]
    fn ads_change_publisher_trust() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };
        conf.publisher = PublisherConfig {
            revenue_per_subscriber: 10.,
            base_budget: 10000.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut agent = Agent::new(0, &conf.agent, &mut rng);
        agent.values.set(Values::from_vec(vec![0., 0.]));
        agent.interests = Topics::from_vec(vec![1., 1.]);
        agent.location = (0, 0);
        agent.relevancies.push(1.0); // Publisher a
        agent.relevancies.push(1.0); // Publisher b

        // Control for location
        let pub_a_id = 0;
        let mut publisher_a = Publisher::new(pub_a_id, &conf.publisher, &mut rng);
        publisher_a.location = (0, 0);
        publisher_a.radius = 1;

        let pub_b_id = 1;
        let mut publisher_b = Publisher::new(pub_b_id, &conf.publisher, &mut rng);
        publisher_b.location = (0, 0);
        publisher_b.radius = 1;

        let author_id = 1;
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(if i < 50 {
                        pub_a_id
                    } else {
                        pub_b_id
                    }),
                    author: author_id,
                    body: ContentBody {
                        topics: Topics::from_vec(vec![1., 1.]),
                        values: Values::from_vec(vec![0., 0.]),
                        cost: 10.,
                        quality: 1.,
                    },
                    ads: if i < 50 {
                        0.
                    } else {
                        10.
                    }
                };
                SharedContent {
                    content: Rc::new(content),
                    sharer: (SharerType::Agent, author_id)
                }
            }).collect();

            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = content.iter()
                .map(|c| (None, c)).collect();
            shared.shuffle(&mut rng);
            agent.consume(&shared, &conf, &mut rng);
        }

        let trust = agent.publishers.borrow();
        let (pub_a_trust, _) = trust.get(&pub_a_id).unwrap();
        let (pub_b_trust, _) = trust.get(&pub_b_id).unwrap();
        println!("trust of p_a:{:?} p_b:{:?}", pub_a_trust, pub_b_trust);
        assert_eq!(*pub_a_trust, 1.0);
        assert!(*pub_b_trust < 0.1);
    }

    #[test]
    fn publisher_publish_to_audience_tastes() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };
        conf.publisher = PublisherConfig {
            revenue_per_subscriber: 10.,
            base_budget: 10000.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);

        // Dummy
        let mut author = Agent::new(0, &conf.agent, &mut rng);

        let mut publisher = Publisher::new(0, &conf.publisher, &mut rng);
        let mut audience = Audience::new(&mut rng);
        let var = Vector::from_vec(vec![0.5, 0.5]);
        audience.values = (Values::from_vec(vec![1., 1.]), var.clone());
        audience.interests = (Topics::from_vec(vec![0., 1.]), var.clone());
        publisher.audience = audience;

        let mut match_accepted = 0;
        let mut similar_accepted = 0;
        let mut not_match_accepted = 0;
        let mut pitches: Vec<(usize, ContentBody)> = (0..120).map(|i| {
            if i < 40 {
                let body = ContentBody {
                    quality: 0., // So it costs nothing
                    topics: Topics::from_vec(vec![0., 1.]),
                    values: Values::from_vec(vec![1., 1.]),
                    cost: 10.,
                };
                (0, body)
            } else if i < 80 {
                let body = ContentBody {
                    quality: 0., // So it costs nothing
                    topics: Topics::from_vec(vec![0.5, 0.5]),
                    values: Values::from_vec(vec![0., 0.]),
                    cost: 10.,
                };
                (1, body)
            } else {
                let body = ContentBody {
                    quality: 0., // So it costs nothing
                    topics: Topics::from_vec(vec![ 1.,  0.]),
                    values: Values::from_vec(vec![-1., -1.]),
                    cost: 10.,
                };
                (2, body)
            }
        }).collect();
        pitches.shuffle(&mut rng);
        for (mtch, body) in pitches {
            match publisher.pitch(&body, &mut author, &conf, &mut rng) {
                Some(_) => {
                    if mtch == 0 {
                        match_accepted += 1
                    } else if mtch == 1 {
                        similar_accepted += 1
                    } else {
                        not_match_accepted += 1
                    }
                },
                None => {}
            };
        }
        println!("match:{:?} similar:{:?} not:{:?}", match_accepted, similar_accepted, not_match_accepted);
        assert!(match_accepted > similar_accepted);
        assert!(similar_accepted > not_match_accepted);
        assert!((match_accepted as f32/40.) > 0.9);
        assert!((similar_accepted as f32/40.) > 0.3 && (similar_accepted as f32/40.) < 0.6);
        assert!((not_match_accepted as f32/40.) < 0.1);
    }

    #[test]
    fn publisher_adjust_audience_understanding_by_shares() {
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
        for a in &mut consumers {
            a.relevancies.push(1.0);
        }

        let mut publisher = Publisher::new(0, &conf.publisher, &mut rng);
        let mut audience = Audience::new(&mut rng);
        let var = Vector::from_vec(vec![0.5, 0.5]);
        audience.values = (Values::from_vec(vec![-1., -1.]), var.clone());
        audience.interests = (Topics::from_vec(vec![0., 0.]), var.clone());
        publisher.audience = audience;

        let author_id = consumers.len();
        let vals_expected = Values::from_vec(vec![0., 0.]);
        let ints_expected = Topics::from_vec(vec![1., 1.]);
        let mut shared: Vec<SharedContent> = Vec::new();
        for _ in 0..10 {
            for i in 0..10 {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(publisher.id),
                    author: author_id,
                    body: ContentBody {
                        topics: ints_expected.clone(),
                        values: vals_expected.clone(),
                        cost: 10.,
                        quality: 1.,
                    },
                    ads: 0.
                };
                let content = Rc::new(content);
                publisher.content.push(content.clone());

                let to_share = SharedContent {
                    content: content.clone(),
                    sharer: (SharerType::Agent, author_id)
                };
                shared.push(to_share);
            }

            for a in &consumers {
                let mut s: Vec<(Option<&PlatformId>, &SharedContent)> = shared.iter()
                    .map(|c| (None, c)).collect();
                s.shuffle(&mut rng);
                let (will_share, _, _, _, _) = a.consume(&s, &conf, &mut rng);
                for c in will_share {
                    let to_share = SharedContent {
                        content: c.clone(),
                        sharer: (SharerType::Agent, a.id)
                    };
                    shared.push(to_share);
                }
            }

            publisher.audience_survey(10);
        }

        let max_distance = 0.001;
        let vals = publisher.audience.values.0;
        let ints = publisher.audience.interests.0;
        println!("audience values:{:?} interests:{:?}", vals, ints);
        assert!((vals[0] - vals_expected[0]).abs() <= max_distance && (vals[1] - vals_expected[1]).abs() <= max_distance);
        assert!((ints[0] - ints_expected[0]).abs() <= max_distance && (ints[1] - ints_expected[1]).abs() <= max_distance);
    }

    #[test]
    fn subscribe_to_trusted_publishers() {
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
        for a in &mut consumers {
            // Equally relevant
            a.relevancies.push(1.0); // Publisher a
            a.relevancies.push(1.0); // Publisher b
        }

        // Control for location
        let pub_a_id = 0;
        let mut publisher_a = Publisher::new(pub_a_id, &conf.publisher, &mut rng);
        publisher_a.location = (0, 0);
        publisher_a.radius = 1;

        let pub_b_id = 1;
        let mut publisher_b = Publisher::new(pub_b_id, &conf.publisher, &mut rng);
        publisher_b.location = (0, 0);
        publisher_b.radius = 1;

        let author_id = consumers.len();
        let mut subs: Vec<isize> = vec![0, 0];
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(if i < 50 {
                        pub_a_id
                    } else {
                        pub_b_id
                    }),
                    author: author_id,
                    body: ContentBody {
                        topics: if i < 50 {
                            Topics::from_vec(vec![1., 1.])
                        } else {
                            Topics::from_vec(vec![0., 0.])
                        },
                        values: if i < 50 {
                            Values::from_vec(vec![0., 0.])
                        } else {
                            Values::from_vec(vec![-1., -1.])
                        },
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
                let (_, (new_subs, unsubs), _, _, _) = a.consume(&shared, &conf, &mut rng);
                for pub_id in new_subs {
                    subs[pub_id] += 1;
                }
                for pub_id in unsubs {
                    subs[pub_id] -= 1;
                }
            }
        }

        println!("subs a:{:?} subs b:{:?}", subs[pub_a_id], subs[pub_b_id]);
        assert!(subs[pub_a_id] > subs[pub_b_id]);
        assert_eq!(subs[pub_a_id], consumers.len() as isize);
        assert_eq!(subs[pub_b_id], 0);

        // Unsubscribe as trust declines
        for _ in 0..100 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(if i < 50 {
                        pub_a_id
                    } else {
                        pub_b_id
                    }),
                    author: author_id,
                    body: ContentBody {
                        topics: if i < 50 {
                            Topics::from_vec(vec![0., 0.])
                        } else {
                            Topics::from_vec(vec![1., 1.])
                        },
                        values: if i < 50 {
                            Values::from_vec(vec![-1., -1.])
                        } else {
                            Values::from_vec(vec![0., 0.])
                        },
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
                let (_, (new_subs, unsubs), _, _, _) = a.consume(&shared, &conf, &mut rng);
                for pub_id in new_subs {
                    subs[pub_id] += 1;
                }
                for pub_id in unsubs {
                    subs[pub_id] -= 1;
                }
            }
        }

        println!("subs a:{:?} subs b:{:?}", subs[pub_a_id], subs[pub_b_id]);
        assert!(subs[pub_b_id] > subs[pub_a_id]);
        assert_eq!(subs[pub_a_id], 0);
        assert_eq!(subs[pub_b_id], consumers.len() as isize);
    }

    #[test]
    fn unsubscribe_from_inactive_publishers() {
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
        for a in &mut consumers {
            a.relevancies.push(1.0);
        }

        // Control for location
        let mut publisher_a = Publisher::new(0, &conf.publisher, &mut rng);
        publisher_a.location = (0, 0);
        publisher_a.radius = 1;

        let author_id = consumers.len();
        let mut subs: Vec<isize> = vec![0];
        for _ in 0..10 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(0),
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
                let (_, (new_subs, unsubs), _, _, _) = a.consume(&shared, &conf, &mut rng);
                for pub_id in new_subs {
                    subs[pub_id] += 1;
                }
                for pub_id in unsubs {
                    subs[pub_id] -= 1;
                }
            }
        }
        assert_eq!(subs[0], consumers.len() as isize);

        let shared = vec![];
        for _ in 0..100 {
            for a in &consumers {
                let (_, (new_subs, unsubs), _, _, _) = a.consume(&shared, &conf, &mut rng);
                for pub_id in new_subs {
                    subs[pub_id] += 1;
                }
                for pub_id in unsubs {
                    subs[pub_id] -= 1;
                }
            }
        }
        assert_eq!(subs[0], 0);
    }

    #[test]
    fn publisher_ad_revenue() {
        let mut conf = SimulationConfig::default();
        conf.revenue_per_ad = 1.;
        conf.agent = AgentConfig {
            attention_budget: 100.
        };
        conf.publisher = PublisherConfig {
            revenue_per_subscriber: 10.,
            base_budget: 10000.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut consumers = standard_agents(&conf, &mut rng);
        for a in &mut consumers {
            a.relevancies.push(1.0);
        }

        // Control for location
        let mut publisher = Publisher::new(0, &conf.publisher, &mut rng);
        publisher.location = (0, 0);
        publisher.radius = 1;

        let mut ad_revenue = 0.;
        let author_id = consumers.len();
        for _ in 0..10 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(0),
                    author: author_id,
                    body: ContentBody {
                        topics: Topics::from_vec(vec![1., 1.]),
                        values: Values::from_vec(vec![0., 0.]),
                        cost: 10.,
                        quality: 1.,
                    },
                    ads: 1.
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
                let (_, _, _, _, revenue) = a.consume(&shared, &conf, &mut rng);
                ad_revenue += revenue.get(&(SharerType::Publisher, publisher.id)).unwrap();
            }
        }
        println!("ad revenue:{:?}", ad_revenue);
        assert_eq!(ad_revenue, 1. * 1. * 10. * 100.);
    }

    #[test]
    fn publisher_ad_market_platform_impact() {
        let mut conf = SimulationConfig::default();
        conf.revenue_per_ad = 1.;
        conf.max_conversion_rate = 0.05;
        conf.base_conversion_rate = 0.01;
        conf.agent = AgentConfig {
            attention_budget: 100.
        };
        conf.publisher = PublisherConfig {
            revenue_per_subscriber: 10.,
            base_budget: 10000.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut consumers = standard_agents(&conf, &mut rng);
        for a in &mut consumers {
            a.relevancies.push(1.0);
        }

        // Control for location
        let pub_id = 0;
        let mut publisher = Publisher::new(pub_id, &conf.publisher, &mut rng);
        publisher.location = (0, 0);
        publisher.radius = 1;
        publisher.ads = 10.;

        let publishers = vec![publisher];
        let platforms = vec![];

        let mut ad_revenue = 0.;
        let author_id = consumers.len();
        for _ in 0..10 {
            let content: Vec<Content> = (0..100).map(|i| {
                Content {
                    id: ContentId::new_v4(),
                    publisher: Some(pub_id),
                    author: author_id,
                    body: ContentBody {
                        topics: Topics::from_vec(vec![1., 1.]),
                        values: Values::from_vec(vec![0., 0.]),
                        cost: 10.,
                        quality: 1.,
                    },
                    ads: 0.
                }
            }).collect();

            let mut c: FnvHashMap<(SharerType, usize), Vec<Content>> = FnvHashMap::default();
            c.insert((SharerType::Publisher, pub_id), content);

            ad_market(&mut c, &consumers, &publishers, &platforms, &conf, &mut rng);

            let content = c.remove(&(SharerType::Publisher, pub_id)).unwrap();
            let shared_content: Vec<SharedContent> = content.into_iter().map(|c| SharedContent {
                content: Rc::new(c),
                sharer: (SharerType::Agent, author_id)
            }).collect();

            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = shared_content.iter()
                .map(|c| (None, c)).collect();
            for a in &consumers {
                shared.shuffle(&mut rng);
                let (_, _, _, _, revenue) = a.consume(&shared, &conf, &mut rng);
                ad_revenue += revenue.get(&(SharerType::Publisher, pub_id)).unwrap();
            }
        }
        println!("ad revenue:{:?}", ad_revenue);
        assert!(ad_revenue > 900.);

        let mut platform = Platform::new(0);
        platform.data = 100000.;
        platform.update_conversion_rate(conf.max_conversion_rate);
        let platforms = vec![platform];

        let mut new_ad_revenue = 0.;
        let author_id = consumers.len();
        for _ in 0..10 {
            let content: Vec<Content> = (0..100).map(|i| {
                Content {
                    id: ContentId::new_v4(),
                    publisher: Some(pub_id),
                    author: author_id,
                    body: ContentBody {
                        topics: Topics::from_vec(vec![1., 1.]),
                        values: Values::from_vec(vec![0., 0.]),
                        cost: 10.,
                        quality: 1.,
                    },
                    ads: 0.
                }
            }).collect();

            let mut c: FnvHashMap<(SharerType, usize), Vec<Content>> = FnvHashMap::default();
            c.insert((SharerType::Publisher, pub_id), content);

            ad_market(&mut c, &consumers, &publishers, &platforms, &conf, &mut rng);

            let content = c.remove(&(SharerType::Publisher, pub_id)).unwrap();
            let shared_content: Vec<SharedContent> = content.into_iter().map(|c| SharedContent {
                content: Rc::new(c),
                sharer: (SharerType::Agent, author_id)
            }).collect();

            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = shared_content.iter()
                .map(|c| (None, c)).collect();
            for a in &consumers {
                shared.shuffle(&mut rng);
                let (_, _, _, _, revenue) = a.consume(&shared, &conf, &mut rng);
                new_ad_revenue += revenue.get(&(SharerType::Publisher, pub_id)).unwrap();
            }
        }
        println!("new ad revenue:{:?}", new_ad_revenue);
        assert!(ad_revenue > new_ad_revenue);
        assert!(new_ad_revenue < 200.);
    }

    #[test]
    fn follow_trusted() {
        let mut conf = SimulationConfig::default();
        conf.agent = AgentConfig {
            attention_budget: 100.
        };

        let mut rng: StdRng = SeedableRng::seed_from_u64(0);
        let mut consumers = standard_agents(&conf, &mut rng);
        for a in &mut consumers {
            a.relevancies.push(1.0);
        }

        let author_id = consumers.len();
        let sharer_id = author_id + 1;

        // Assume all consumers follow sharer initially
        let mut followers = vec![0, consumers.len() as isize];
        for _ in 0..10 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(0),
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
                    sharer: (SharerType::Agent, sharer_id)
                }
            }).collect();

            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = content.iter()
                .map(|c| (None, c)).collect();
            for a in &consumers {
                shared.shuffle(&mut rng);
                let (_, _, (follows, unfollows), _, _) = a.consume(&shared, &conf, &mut rng);
                for id in follows {
                    if id == author_id {
                        followers[0] += 1;
                    } else {
                        followers[1] += 1;
                    }
                }
                for id in unfollows {
                    if id == author_id {
                        followers[0] -= 1;
                    } else {
                        followers[1] -= 1;
                    }
                }
            }
        }

        // May be redundant follows b/c we aren't checking if already following
        assert!(followers[0] >= consumers.len() as isize);
        assert_eq!(followers[1], consumers.len() as isize); // Shouldn't have changed

        for _ in 0..10 {
            let content: Vec<SharedContent> = (0..100).map(|i| {
                let content = Content {
                    id: ContentId::new_v4(),
                    publisher: Some(0),
                    author: author_id,
                    body: ContentBody {
                        topics: Topics::from_vec(vec![ 0., 0.]),
                        values: Values::from_vec(vec![-1., -1.]),
                        cost: 10.,
                        quality: 1.,
                    },
                    ads: 0.
                };
                SharedContent {
                    content: Rc::new(content),
                    sharer: (SharerType::Agent, sharer_id)
                }
            }).collect();

            let mut shared: Vec<(Option<&PlatformId>, &SharedContent)> = content.iter()
                .map(|c| (None, c)).collect();
            for a in &consumers {
                shared.shuffle(&mut rng);
                let (_, _, (follows, unfollows), _, _) = a.consume(&shared, &conf, &mut rng);
                for id in follows {
                    if id == author_id {
                        followers[0] += 1;
                    } else {
                        followers[1] += 1;
                    }
                }
                for id in unfollows {
                    if id == author_id {
                        followers[0] -= 1;
                    } else {
                        followers[1] -= 1;
                    }
                }
            }
        }

        // May be redundant unfollows b/c we aren't checking if already not following
        assert!(followers[0] <= 0);
        assert!(followers[1] <= 0);
    }
}
