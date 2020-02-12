use serde::Deserialize;
use redis::{Commands, Connection};
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
    Reset,                          //
}

pub enum Command {
    Run(usize),
    Reset
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

    fn set_status(&self, state: Status) -> redis::RedisResult<()> {
        self.con.set("status", state.to_string().to_lowercase())?;
        Ok(())
    }

    pub fn set_ready(&self) -> redis::RedisResult<()> {
        self.set_status(Status::Ready)
    }

    pub fn set_running(&self) -> redis::RedisResult<()> {
        self.set_status(Status::Running)
    }

    pub fn set_loading(&self) -> redis::RedisResult<()> {
        self.set_status(Status::Loading)
    }

    pub fn reset(&mut self) -> redis::RedisResult<()> {
        self.con.del("cmds") // Clear queued commands
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
                        Message::Reset => {
                            command = Some(Command::Reset);
                        }
                    }
                }
            }
        }
        command
    }
}
