use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{EventId, UserId};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub events: HashSet<EventId>,
}
impl User {
    pub fn new(id: UserId, name: &str) -> Self {
        Self {
            id,
            name: String::from(name),
            events: HashSet::new(),
        }
    }
}
impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for User {}
