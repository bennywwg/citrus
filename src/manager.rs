use std::{collections::HashSet};

use crate::{component::Component, entity::*};

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
            let mut ent_addr = self.entities[index].make_addr();

            {
                let mut component_index = 0 as usize;
                let mut components = ent_addr.get_ref_mut().unwrap().erased_components();
                while component_index < components.len() {
                    components[component_index].get_ref_mut().unwrap().update(self, ent_addr.clone());
                    component_index += 1;
                }
            }

            
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
    pub fn of_type<T: Component>() -> Vec<EntAddr> {
        todo!();
    }
    pub fn create_entity(&mut self) -> EntAddr {
        self.entities.push(EntityHolder::new());
        self.entities.last_mut().unwrap().make_addr()
    }
    pub fn destroy_entity(&mut self, addr: EntAddr) {
        self.destroy_queue.insert(addr);
    }
}