use std::{collections::HashSet, ops::Index};

use crate::entity::*;

pub struct Manager {
    entities: Vec<EntityHolder>,
    destroy_queue: HashSet<EntAddr>
}

impl Manager {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            destroy_queue: HashSet::new()
        }
    }
    // TODO: Make this a non-mut function
    fn find_ent_index(&mut self, addr: &EntAddr) -> Option<usize> {
        self.entities.iter_mut().position(|ent| ent.make_addr().eq(addr))
    }
    pub fn update(&mut self) {
        let mut index = 0 as usize;
        while index < self.entities.len() {
            let ent = &mut self.entities[index];
            for component in ent.make_addr().get_ref_mut().unwrap().components_iter_mut() {
                component.get_dyn_ref_mut().update();
            }

            // TODO: Make this cleaner if find_ent_index is made non-mut
            let cloned_destroy_queue = self.destroy_queue.clone();
            self.destroy_queue.clear();
            for to_destroy in cloned_destroy_queue.iter() {
                if let Some(destroy_index) = self.find_ent_index(to_destroy) {
                    self.entities.remove(destroy_index);
                }
            }
            index += 1;
        }
    }
    pub fn create_entity(&mut self) -> EntAddr {
        self.entities.push(EntityHolder::new());
        self.entities.last_mut().unwrap().make_addr()
    }
    pub fn destroy_entity(&mut self, addr: EntAddr) {
        self.destroy_queue.insert(addr);
    }
}