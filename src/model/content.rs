use std::rc::Rc;
use super::agent::{Topics, Values, AgentId};
use super::publisher::PublisherId;

#[derive(Debug)]
pub struct Content {
    pub publisher: Option<PublisherId>,
    pub author: AgentId,
    pub body: ContentBody,
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
    pub sharer: AgentId
}
