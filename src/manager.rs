use crate::entity::*;

pub struct Manager {
    entities: Vec<EntityHolder>
}

impl Manager {
    pub fn new() -> Self {
        Self {
            entities: Vec::new()
        }
    }
    pub fn update(&mut self) {
        
    }
    pub fn create_entity(&mut self) -> EntAddr {
        self.entities.push(EntityHolder::new());
        self.entities.last_mut().unwrap().make_addr()
    }
}