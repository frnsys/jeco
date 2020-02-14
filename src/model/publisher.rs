// TODO
// - implement pitching
//  - [X] publisher deciding when to accept a pitch
//  - [X] publisher deciding how much to invest in a pitch
//  - agent deciding when and where to pitch
//      - based on pitch topics/values
//      - EV calculation of expected reach and prob of accepting
// - implement publishing
//  - [X] add content to all subscriber inboxes
//  - add content through agent networks/non-subscriber readers
// - [X] implement audience interest/value tracking (EWMAs et al)
// - implement subscribing
//  - agents deciding when to subscribe
//      - need to consider getting content for free vs paying
//
// After ads are implemented
// - deciding on how many ads to include
//  - civic vs profit-driven utility mixture

use rand::Rng;
use std::rc::Rc;
use rand::rngs::StdRng;
use std::f32::consts::E;
use itertools::Itertools;
use super::agent::{Agent, AgentId, Vector};
use super::content::{Content, ContentBody, SharedContent, SharerType};

static REVENUE_PER_SUBSCRIBER: f32 = 0.01;

pub type PublisherId = usize;

// Bayesian normal update
use nalgebra::{Matrix, Dynamic, U2, VecStorage, RowVectorN};
type Sample = Matrix<f32, Dynamic, U2, VecStorage<f32, Dynamic, U2>>;
pub fn bayes_update(prior: (Vector, Vector), sample: Sample) -> (Vector, Vector) {
    let (prior_mu, prior_var) = prior;
    let sample_mu = sample.column_mean();
    let sample_var = sample.column_variance();
    let denom = prior_var + &sample_var;
    let post_mu = (sample_var.component_mul(&prior_mu) + prior_var.component_mul(&sample_mu)).component_div(&denom);
    let post_var = sample_var.component_mul(&prior_var).component_div(&denom);
    (post_mu, post_var)
}


// A Publisher is a platform which
// exercises discretion of what
// content circulates through it.
// When content is circulated from a Publisher,
// it carries the reputation of the Publisher.
// (In contrast, on a Platform the content
// that circulates through it carries
// the reputation of the sender)
#[derive(Debug)]
pub struct Publisher {
    pub id: PublisherId,

    // Budget determines how much content
    // can be published per step
    // and at what quality.
    budget: f32,

    // The content quality the Publisher
    // aims for. Could be replaced with something
    // more sophisticated.
    quality: f32,

    // Agents subscribed to the publication.
    // These count towards the Publisher's overall budget
    // and directly received the Publisher's content
    pub subscribers: Vec<AgentId>,

    // Store content the Publisher will
    // publish in the next step. Emptied each step.
    pub outbox: Vec<SharedContent>,

    // Archive of content the Publisher
    // has published
    pub content: Vec<Rc<Content>>,

    // Publisher tries to guess
    // the distribution of their
    // audiences' values and interests
    pub audience_values: (Vector, Vector),
    pub audience_interests: (Vector, Vector),
}

impl Publisher {
    pub fn new(id: PublisherId, rng: &mut StdRng) -> Publisher {
        let mu = Vector::from_vec(vec![0., 0.]);
        let var = Vector::from_vec(vec![1., 1.]);

        Publisher {
            id: id,
            budget: 0.,
            quality: rng.gen(),
            outbox: Vec::new(),
            content: Vec::new(),
            subscribers: Vec::new(),

            // Priors
            audience_values: (mu.clone(), var.clone()),
            audience_interests: (mu.clone(), var.clone()),
        }
    }

    // An Agent pitches a piece
    // of content to the publisher
    pub fn pitch(&mut self, body: &ContentBody, author: &Agent, rng: &mut StdRng) -> bool {
        if self.budget < self.quality { return false }

        // TODO
        let sim_to_perceived_reader = 0.5;

        // Sigmoid
        let p_accept = 1./(1.+E.powf(-sim_to_perceived_reader-0.5));
        let roll: f32 = rng.gen();
        let accepted = roll < p_accept;
        if accepted {
            let content = Rc::new(Content {
                publisher: Some(self.id),
                body: *body,
                author: author.id,
            });
            self.content.push(content.clone());
            self.outbox.push(SharedContent {
                content: content.clone(),
                sharer: (SharerType::Publisher, self.id)
            });

            // Deduct from budget
            self.budget -= self.quality;
        }
        accepted
    }

    // An Agent subscribes to the publisher
    pub fn subscribe(&mut self, agent: &Agent) {
        self.subscribers.push(agent.id);
    }

    fn operating_budget(&self) -> f32 {
        self.subscribers.len() as f32 * REVENUE_PER_SUBSCRIBER
    }

    // Update understanding of audience values/interests
    // ENH: Can be smarter about how we pick the content
    // to look at. Ideally also weight content by shares.
    pub fn audience_survey(&mut self, sample_size: usize) {
        let mut v_rows: Vec<RowVectorN<f32, U2>> = Vec::with_capacity(sample_size);
        let mut i_rows: Vec<RowVectorN<f32, U2>> = Vec::with_capacity(sample_size);
        for c in self.content_by_popularity().take(sample_size) {
            v_rows.push(c.body.values.transpose());
            i_rows.push(c.body.topics.transpose());
        }
        let mut sample = Sample::from_rows(v_rows.as_slice());
        self.audience_values = bayes_update(self.audience_values, sample);

        sample = Sample::from_rows(i_rows.as_slice());
        self.audience_interests = bayes_update(self.audience_interests, sample);
    }

    pub fn content_by_popularity(&self) -> std::vec::IntoIter<&Rc<Content>> {
        self.content.iter().sorted_by(|a, b| Rc::strong_count(b).cmp(&Rc::strong_count(a)))
    }
}
