use std::fs;
use std::path::Path;
use std::os::unix::fs::symlink;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use super::config::Config;
use super::network::Network;
use std::rc::Rc;
use super::agent::Agent;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

pub struct Recorder {
    history: Vec<Value>,
    sample: Vec<Rc<Agent>>,
}

impl Recorder {
    pub fn new(network: &Network, mut rng: &mut StdRng) -> Recorder {
        let sample_size = 20;
        Recorder {
            history: Vec::new(),
            sample: network.agents.choose_multiple(&mut rng, sample_size).map(|a| a.clone()).collect()
        }
    }

    pub fn record(&mut self, network: &Network) {
        let sample: Vec<Value> = self.sample.iter().map(|a| {
            json!({
                "id": a.id,
                "values": a.values,
                "interests": a.interests,
            })
        }).collect();
        let value = json!({
            "sample": sample
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
}
