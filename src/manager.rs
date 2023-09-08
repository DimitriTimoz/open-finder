use crate::prelude::*;
use std::collections::HashSet;

pub struct LinkCollection {
    insa_links: HashSet<Url>,
    others: HashSet<Url>
}

impl LinkCollection {
    pub fn add_insa_link(&mut self, url: Url) {
        self.insa_links.insert(url);
    }

    pub fn add_other_link(&mut self, url: Url) {
        self.others.insert(url);
    }

    pub fn contains(&self, url: Url) -> bool {
        self.insa_links.contains(&url) || self.others.contains(&url)
    }

    pub fn get_insa_urls(&self) -> impl Iterator<Item = &Url> {
        self.insa_links.iter()
    }
}