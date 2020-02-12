use rand::Rng;
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub struct Config {
    pub population: usize,

    #[serde(default)]
    pub steps: usize,

    #[serde(default)]
    pub debug: bool,

    #[serde(default)]
    pub command: bool,

    #[serde(default)]
    pub seed: u64,
}

pub fn load_config() -> Config {
    let file = File::open("config.yaml").expect("could not open file");
    let reader = BufReader::new(file);
    let mut conf: Config = serde_yaml::from_reader(reader).expect("error while reading yaml");

    conf.steps = match env::var("STEPS") {
        Ok(steps) => steps.parse().unwrap(),
        Err(_) => 100,
    };

    conf.debug = match env::var("DEBUG") {
        Ok(debug) => debug == "1",
        Err(_) => conf.debug,
    };

    conf.command = match env::var("COMMAND") {
        Ok(command) => command == "1",
        Err(_) => conf.command,
    };

    let mut rng = rand::thread_rng();
    conf.seed = match env::var("SEED") {
        Ok(seed) => seed.parse().unwrap(),
        Err(_) => rng.gen(),
    };

    println!("{:?}", conf);

    conf
}
