use super::model::{Simulation, Agent, AgentId, Publisher, PublisherId, Values};
use super::config::Config;
use chrono::{DateTime, Utc};
use fnv::FnvHashMap;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::rc::Rc;
use redis::Commands;

pub struct Recorder {
    history: Vec<Value>,
    agents: Vec<AgentId>,
    publishers: Vec<PublisherId>,
    init_values: Vec<Values>,
}

pub fn mean_usize(vec: &Vec<usize>) -> f32 {
    vec.iter()
        .fold(0, |acc, v| acc + v) as f32 / vec.len() as f32
}
pub fn mean_f32(vec: &Vec<f32>) -> f32 {
    vec.iter()
        .fold(0., |acc, v| acc + v) / vec.len() as f32
}
pub fn max_f32(vec: &Vec<f32>) -> f32 {
    vec.iter().fold(-1./0., |a, &b| f32::max(a, b))
}
pub fn min_f32(vec: &Vec<f32>) -> f32 {
    vec.iter().fold(1./0., |a, &b| f32::min(a, b))
}


impl Recorder {
    pub fn new(sim: &Simulation, mut rng: &mut StdRng) -> Recorder {
        let a_sample_size = (0.2 * sim.agents.len() as f32) as usize;
        let agents: Vec<AgentId> = sim.agents
            .choose_multiple(&mut rng, a_sample_size)
            .map(|a| a.id)
            .collect();
        let init_values = agents.iter().map(|id| sim.agents[*id].values.get()).collect();

        let p_sample_size = 10;
        let publishers: Vec<PublisherId> = sim.publishers
            .choose_multiple(&mut rng, p_sample_size)
            .map(|p| p.id)
            .collect();

        Recorder {
            history: Vec::new(),
            agents: agents,
            publishers: publishers,
            init_values: init_values
        }
    }

    pub fn record(&mut self, step: usize, sim: &Simulation, n_produced: usize) {
        let agents: Vec<&Agent> = self.agents.iter().map(|id| &sim.agents[*id]).collect();
        let a_sample: Vec<Value> = agents
            .iter()
            .map(|a| {
                json!({
                    "id": a.id,
                    "values": a.values,
                    "interests": a.interests,
                })
            })
            .collect();

        let publishers: Vec<&Publisher> = self.publishers.iter().map(|id| &sim.publishers[*id]).collect();
        let p_sample: Vec<Value> = publishers
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "values": p.audience_values.0, // mean only
                    "interests": p.audience_interests.0, // ditto
                })
            })
            .collect();

        let platforms = sim.platforms.iter().fold(FnvHashMap::default(), |mut acc, p| {
            acc.insert(p.id, json!({
                "users": p.n_users(),
                "data": p.data,
            }));
            acc
        });

        // Top 10
        let content: Vec<Value> = sim.content_by_popularity().take(10).map(|c| {
            json!({
                "shares": Rc::strong_count(c) - 1,
                "topics": c.body.topics,
                "values": c.body.values
            })
        }).collect();

        let value_shifts: Vec<f32> = agents.iter().zip(self.init_values.iter())
            .map(|(a, b)| 1. - a.values.get().normalize().dot(&b.normalize())).collect();

        let n_shares = sim.n_shares();
        let mut share_dist: FnvHashMap<usize, usize> = FnvHashMap::default();
        for shares in &n_shares {
            let count = share_dist.entry(*shares).or_insert(0);
            *count += 1;
        }

        let n_followers = sim.network.n_followers();
        let mut follower_dist: FnvHashMap<usize, usize> = FnvHashMap::default();
        for followers in &n_followers {
            let count = follower_dist.entry(*followers).or_insert(0);
            *count += 1;
        }

        let n_subscribers: Vec<usize> = sim.publishers.iter().map(|p| p.subscribers).collect();
        let n_published: Vec<usize> = sim.publishers.iter().map(|p| p.n_last_published).collect();
        let reach: Vec<f32> = sim.publishers.iter().map(|p| p.reach).collect();
        let budget: Vec<f32> = sim.publishers.iter().map(|p| p.budget).collect();
        let publishability: Vec<f32> = agents.iter().map(|a| a.publishability).collect();

        let value = json!({
            "step": step,
            "shares": {
                "max": n_shares.iter().max(),
                "min": n_shares.iter().min(),
                "mean": mean_usize(&n_shares),
            },
            "share_dist": share_dist,
            "followers": {
                "max": n_followers.iter().max(),
                "min": n_followers.iter().min(),
                "mean": mean_usize(&n_followers),
            },
            "follower_dist": follower_dist,
            "value_shifts": {
                "max": max_f32(&value_shifts),
                "min": min_f32(&value_shifts),
                "mean": mean_f32(&value_shifts),
            },
            "subscribers": {
                "max": n_subscribers.iter().max(),
                "min": n_subscribers.iter().min(),
                "mean": mean_usize(&n_subscribers),
            },
            "published": {
                "max": n_published.iter().max(),
                "min": n_published.iter().min(),
                "mean": mean_usize(&n_published),
            },
            "reach": {
                "max": max_f32(&reach),
                "min": min_f32(&reach),
                "mean": mean_f32(&reach),
            },
            "budget": {
                "max": max_f32(&budget),
                "min": min_f32(&budget),
                "mean": mean_f32(&budget),
            },
            "publishability": {
                "max": max_f32(&publishability),
                "min": min_f32(&publishability),
                "mean": mean_f32(&publishability),
            },
            "agents": a_sample,
            "publishers": p_sample,
            "platforms": platforms,
            "p_produced": n_produced as f32/sim.agents.len() as f32,
            "to_share": sim.n_will_share(),
            "top_content": content
        });
        self.history.push(value);
    }

    pub fn save(&self, conf: &Config) {
        let now: DateTime<Utc> = Utc::now();
        let now_str = now.format("%Y.%m.%d.%H.%M.%S").to_string();
        let results = json!({
            "history": self.history,
            "meta": {
                "seed": conf.seed,
                "steps": conf.steps,
                "conf": conf.simulation,
            }
        })
        .to_string();

        let dir = format!("runs/{}", now_str);
        let fname = format!("runs/{}/output.json", now_str);

        let path = Path::new(&dir);
        let run_path = Path::new(&now_str);
        let latest_path = Path::new("runs/latest");
        fs::create_dir(path).unwrap();
        fs::write(fname, results).expect("Unable to write file");
        if latest_path.exists() {
            fs::remove_file(latest_path).unwrap();
        }
        symlink(run_path, latest_path).unwrap();

        let conf_path = Path::join(path, Path::new("config.yaml"));
        fs::copy(Path::new("config.yaml"), conf_path).unwrap();
        println!("Wrote output to {:?}", path);
    }

    pub fn sync(&self, step: usize, redis_host: &str) -> redis::RedisResult<()> {
        match self.history.get(step) {
            None => (),
            Some(snapshot) => {
                let client = redis::Client::open(redis_host)?;
                let mut con = client.get_connection()?;

                let state_serialized = snapshot.to_string();
                con.rpush("state:history", state_serialized)?;
                con.set("state:step", format!("{:?}", step))?;
            }
        }
        Ok(())
    }
}
