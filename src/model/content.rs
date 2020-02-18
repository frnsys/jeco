use std::rc::Rc;
use super::agent::{Topics, Values, AgentId};
use super::publisher::PublisherId;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SharerType {
    Agent,
    Publisher
}

#[derive(Debug)]
pub struct Content {
    pub publisher: Option<PublisherId>,
    pub author: AgentId,
    pub body: ContentBody,
    pub ads: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct ContentBody {
    pub cost: f32,
    pub topics: Topics,
    pub values: Values,
}

#[derive(Debug)]
pub struct SharedContent {
    pub content: Rc<Content>,
    pub sharer: (SharerType, usize),
}
