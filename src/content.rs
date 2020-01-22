use super::agent::Agent;

#[derive(Debug)]
pub struct Content<'a> {
    pub owner: &'a Agent,
    pub cost: f32
}

