use serde::Deserialize;
use redis::{Commands, Connection};
use super::config::Config;
use strum_macros::{Display};

#[derive(Display, Debug)]
pub enum Status {
    Loading,
    Ready,
    Running,
}

#[derive(Display, PartialEq, Debug, Deserialize)]
enum Message {
    Run(usize),                     // steps
    Reset(Config),
}

pub enum Command {
    Run(usize),
    Reset(Config)
}


pub struct Commander {
    con: Connection,
}

impl Commander {
    pub fn new(redis_host: &str) -> Commander {
        let client = redis::Client::open(redis_host).unwrap();
        let con = client.get_connection().unwrap();

        Commander {
            con: con,
        }
    }

    fn set_status(&mut self, state: Status) -> redis::RedisResult<()> {
        self.con.set("status", state.to_string().to_lowercase())?;
        Ok(())
    }

    pub fn set_ready(&mut self) -> redis::RedisResult<()> {
        self.set_status(Status::Ready)
    }

    pub fn set_running(&mut self) -> redis::RedisResult<()> {
        self.set_status(Status::Running)
    }

    pub fn set_loading(&mut self) -> redis::RedisResult<()> {
        self.set_status(Status::Loading)
    }

    pub fn reset(&mut self, conf: &Config) -> redis::RedisResult<()> {
        self.con.del("cmds")?;
        self.con.del("state:history")?;
        self.con.set("state:step", -1)?;

        let conf_serialized = serde_json::to_string(conf).unwrap();
        self.con.set("config", conf_serialized)
    }

    pub fn wait_for_command(&mut self) -> Command {
        loop {
            let command = self.process_commands();
            match command {
                Some(ctrl) => return ctrl,
                None => continue
            }
        }
    }

    pub fn process_commands(&mut self) -> Option<Command> {
        let mut command = None;
        loop {
            let cmd_raw: Option<String> = self.con.lpop("cmds").unwrap();
            match cmd_raw {
                None => break,
                Some(cmd) => {
                    match serde_json::from_str(&cmd).unwrap() {
                        Message::Run(n) => {
                            command = Some(Command::Run(n));
                        },
                        Message::Reset(overrides) => {
                            command = Some(Command::Reset(overrides));
                        }
                    }
                }
            }
        }
        command
    }
}
