use uuid::Uuid;
use std::rc::Rc;
use super::agent::{Topics, Values, AgentId};
use super::publisher::PublisherId;

pub type ContentId = Uuid;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SharerType {
    Agent,
    Publisher
}

#[derive(Debug)]
pub struct Content {
    pub id: Uuid,
    pub publisher: Option<PublisherId>,
    pub author: AgentId,
    pub body: ContentBody,
    pub ads: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct ContentBody {
    pub cost: f32,
    pub quality: f32,
    pub topics: Topics,
    pub values: Values,
}

#[derive(Debug)]
pub struct SharedContent {
    pub content: Rc<Content>,
    pub sharer: (SharerType, usize),
}
