// TODO
// After ads are implemented
// - deciding on how many ads to include
//  - civic vs profit-driven utility mixture
//
// After social networks are implemented
// - circulate content through agent networks/non-subscriber readers

use rand::Rng;
use std::rc::Rc;
use rand::rngs::StdRng;
use itertools::Itertools;
use super::agent::Agent;
use super::content::{Content, ContentId, ContentBody, SharedContent};
use super::util::{Vector, Params, Sample, SampleRow, ewma, bayes_update, z_score, sigmoid, learn_steps, LimitedQueue};
use super::config::PublisherConfig;

pub type PublisherId = usize;

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
    pub budget: f32,
    revenue_per_subscriber: f32,

    // The content quality the Publisher
    // aims for. Could be replaced with something
    // more sophisticated.
    pub quality: f32,

    // How many ads the Publisher uses
    pub ads: f32,

    // Params for estimating quality/ads mix
    theta: Params,
    observations: Vec<f32>,
    outcomes: Vec<f32>,

    // A Publisher's "reach" is the mean shared
    // count of its content per step
    pub reach: f32,

    // Number of agents subscribed to the publication.
    // These count towards the Publisher's overall budget
    // and directly received the Publisher's content
    pub subscribers: usize,

    // How much was published in the last step
    pub n_last_published: usize,

    // How many ads were sold in the last step
    pub n_ads_sold: f32,

    // Archive of content the Publisher
    // has published
    pub content: LimitedQueue<Rc<Content>>,

    // Publisher tries to guess
    // the distribution of their
    // audiences' values and interests
    pub audience_values: (Vector, Vector),
    pub audience_interests: (Vector, Vector),
}

impl Publisher {
    pub fn new(id: PublisherId, conf: &PublisherConfig, rng: &mut StdRng) -> Publisher {
        let mu = Vector::from_vec(vec![0., 0.]);
        let var = Vector::from_vec(vec![1., 1.]);

        Publisher {
            id: id,
            budget: conf.base_budget,
            revenue_per_subscriber: conf.revenue_per_subscriber,
            reach: 0.,

            ads: rng.gen::<f32>() * 10.,
            quality: rng.gen::<f32>(),
            theta: Params::new(rng.gen(), rng.gen()),
            observations: Vec::new(),
            outcomes: Vec::new(),
            n_ads_sold: 0.,

            content: LimitedQueue::new(50),
            subscribers: 0,
            n_last_published: 0,

            // Priors
            audience_values: (mu.clone(), var.clone()),
            audience_interests: (mu.clone(), var.clone()),
        }
    }

    // An Agent pitches a piece
    // of content to the publisher
    pub fn pitch(&mut self, body: &ContentBody, author: &Agent, rng: &mut StdRng) -> Option<Content> {
        if self.budget < self.quality { return None }

        let z_ints = z_score(&body.topics, &self.audience_interests);
        let z_vals = z_score(&body.values, &self.audience_values);
        let sim_to_perceived_reader = f32::max(1. - (z_ints.mean() + z_vals.mean())/6., 0.);
        // 2*3=6; 2 for the mean, max z-score of 3

        // TODO this doesn't necessarily need to be random?
        // Could just be based on a threshold
        let p_accept = sigmoid(sim_to_perceived_reader-0.5);
        let roll: f32 = rng.gen();
        let accepted = roll < p_accept;
        if accepted {
            // Publisher improves the quality
            let mut body_ = body.clone();
            body_.quality += self.quality;
            let content = Content {
                id: ContentId::new_v4(),
                publisher: Some(self.id),
                body: body_,
                author: author.id,
                ads: self.ads
            };

            // Deduct from budget
            self.budget -= self.quality;
            Some(content)
        } else {
            None
        }
    }

    pub fn operating_budget(&self) -> f32 {
        self.subscribers as f32 * self.revenue_per_subscriber
    }

    // Update understanding of audience values/interests
    // ENH: Can be smarter about how we pick the content
    // to look at. Ideally also weight content by shares.
    pub fn audience_survey(&mut self, sample_size: usize) {
        if self.content.len() == 0 { return }

        // TODO should these be merged into one "audience" matrix?
        // Might be faster
        let mut v_rows: Vec<SampleRow> = Vec::with_capacity(sample_size);
        let mut i_rows: Vec<SampleRow> = Vec::with_capacity(sample_size);
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

    pub fn n_shares(&self) -> Vec<usize> {
        // -1 to account for reference in self.content
        // -1 to account for reference in Sim's self.content
        self.content.iter().map(|c| Rc::strong_count(c) - 2).collect()
    }

    pub fn update_reach(&mut self) {
        let shares = self.n_shares();
        if shares.len() == 0 {
            self.reach = 0.;
        } else {
            let mean_shares = shares.iter().fold(0, |acc, v| acc + v) as f32 / shares.len() as f32;
            self.reach = ewma(mean_shares, self.reach);
        }
    }

    pub fn learn(&mut self, revenue: f32, change_rate: f32) {
        // Assume reach has been updated
        self.outcomes.push(revenue * self.reach); // TODO more balanced mixture of the two?

        self.observations.push(self.quality);
        self.observations.push(self.ads);

        // TODO don't necessarily need to learn _every_ step.
        self.theta = learn_steps(&self.observations, &self.outcomes, self.theta);
        let steps: Vec<f32> = self.theta.into_iter().cloned().collect();
        self.quality += change_rate * steps[0];
        self.ads += change_rate * steps[1];
        self.ads = f32::max(0., self.ads);
        self.quality = f32::max(0., self.quality);
    }
}
