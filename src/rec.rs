use super::model::{Simulation, Agent};
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
use md5::Digest;
use redis::Commands;

pub struct Recorder {
    history: Vec<Value>,
    sample: Vec<Rc<Agent>>,
}

impl Recorder {
    pub fn new(sim: &Simulation, mut rng: &mut StdRng) -> Recorder {
        let sample_size = (0.2 * sim.agents.len() as f32) as usize;
        Recorder {
            history: Vec::new(),
            sample: sim
                .agents
                .choose_multiple(&mut rng, sample_size)
                .map(|a| a.clone())
                .collect(),
        }
    }

    pub fn record(&mut self, sim: &Simulation, n_produced: usize) {
        let sample: Vec<Value> = self
            .sample
            .iter()
            .map(|a| {
                json!({
                    "id": a.id,
                    "values": a.values,
                    "interests": a.interests,
                })
            })
            .collect();
        let n_shares = sim.n_shares();
        let mean_shares = n_shares.iter().fold(0, |acc, v| acc + v) as f32 / n_shares.len() as f32;
        let mut share_dist: FnvHashMap<usize, usize> = FnvHashMap::default();
        for shares in &n_shares {
            let count = share_dist.entry(*shares).or_insert(0);
            *count += 1;
        }

        let n_followers = sim.network.n_followers();
        let mean_followers =
            n_followers.iter().fold(0, |acc, v| acc + v) as f32 / n_followers.len() as f32;
        let mut follower_dist: FnvHashMap<usize, usize> = FnvHashMap::default();
        for followers in &n_followers {
            let count = follower_dist.entry(*followers).or_insert(0);
            *count += 1;
        }

        let value = json!({
            "shares": {
                "max": n_shares.iter().max(),
                "min": n_shares.iter().min(),
                "mean": mean_shares,
            },
            "share_dist": share_dist,
            "followers": {
                "max": n_followers.iter().max(),
                "min": n_followers.iter().min(),
                "mean": mean_followers,
            },
            "follower_dist": follower_dist,
            "sample": sample,
            "p_produced": n_produced as f32/sim.agents.len() as f32,
            "to_share": sim.n_will_share(),
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
                "population": conf.population,
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

    pub fn snapshot(&self) -> Option<Value> {
        self.history.last().cloned()
    }

    pub fn sync(&self, redis_host: &str) -> redis::RedisResult<()> {
        match self.snapshot() {
            None => (),
            Some(snapshot) => {
                let client = redis::Client::open(redis_host)?;
                let con = client.get_connection()?;

                let state_serialized = snapshot.to_string();
                let hash = md5::Md5::digest(state_serialized.as_bytes());

                con.set("state", state_serialized)?;
                con.set("state:key", format!("{:X}", hash))?;

            }
        }
        Ok(())
    }
}
