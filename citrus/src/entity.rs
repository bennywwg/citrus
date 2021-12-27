use std::{any::TypeId, cell::Cell, ops::{Deref, DerefMut}, rc::{Rc, Weak}};
use std::hash::Hash;
use std::{collections::HashSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::deserialize_context::*;
use crate::element::*;

#[derive(Debug)]
pub struct EntReferenceCycleError;

pub struct Entity {
    elements: Vec<ElementHolder>,
    self_addr: EntAddr,
    parent_addr: EntAddr,
    children_addrs: Vec<EntAddr>,
    id: Uuid,
    pub name: String,
}

impl Entity {
    pub fn get_id(&self) -> Uuid {
        self.id
    }

    // A list of all elements with the type information erased
    pub fn erased_elements(&mut self) ->                        Vec<EleAddrErased> {
        self.elements.iter_mut()
        .map(|holder| holder.make_addr_erased())
        .collect::<Vec<EleAddrErased>>()
    }

    // Create element, can occur at any time
    pub fn add_element<T: Element>(&mut self, val: T) ->        Result<EleAddr<T>, String> {
        if self.query_element_addr::<T>().valid() {
            return Err(format!("Element of type \"{}\" is already present", std::any::type_name::<T>()));
        }
        self.elements.push(ElementHolder::new(val, self.self_addr.clone()));
        Ok(self.query_element_addr::<T>())
    }
    
    // Querying addresses
    pub fn query_element_addr<T: Element>(&mut self) ->         EleAddr<T> {
        for comp in self.elements.iter_mut() {
            if comp.get_element_type_id() == std::any::TypeId::of::<T>() {
                return comp.make_addr::<T>();
            }
        }
        EleAddr::new()
    }
    pub fn query_element_addr_by_id(&mut self, id: &TypeId) ->  EleAddrErased {
        self.elements.iter_mut()
        .find(|ele| ele.get_element_type_id() == *id)
        .map_or(EleAddrErased::new(), |ele| ele.make_addr_erased())
    }
    
    // Querying conveniance functions (just call get_ref/_mut on the address)
    pub fn query_element<T: Element>(&mut self) ->              Option<EleRef<T>> {
        self.query_element_addr().get_ref()
    }
    pub fn query_element_mut<T: Element>(&mut self) ->          Option<EleRefMut<T>> {
        self.query_element_addr().get_ref_mut()
    }
    pub fn query_element_by_id(&mut self, id: &TypeId) ->       Option<EleRefErased> {
        self.query_element_addr_by_id(id).get_ref()
    }
    pub fn query_element_mut_by_id(&mut self, id: &TypeId) ->   Option<EleRefErasedMut> {
        self.query_element_addr_by_id(id).get_ref_mut()
    }

    pub fn get_parent(&self) ->                                 EntAddr {
        self.parent_addr.clone()
    }
    pub fn get_children(&self) ->                               Vec<EntAddr> {
        self.children_addrs.clone()
    }
    pub fn get_all_children(&self) ->                           Vec<EntAddr> {
        todo!();
    }
    
    // Private function used by Manager; noop if the element isn't found
    fn remove_element(&mut self, addr: EleAddrErased) {
        if let Some(element_index)
        =self.elements.iter_mut()
        .position(|ele| ele.make_addr_erased().eq(&addr))
        {
            self.elements.remove(element_index);
        }
    }
}

pub struct EntityHolder {
    data: *mut Entity, // must be cleaned up with a Box::from_raw
    internal: Rc<Cell<i64>>
}

impl EntityHolder {
    pub fn new(name: String) -> Self {
        Self {
            data: Box::into_raw(Box::new(Entity {
                elements: Vec::new(),
                self_addr: EntAddr::new(),
                parent_addr: EntAddr::new(),
                children_addrs: vec!(),
                id: Uuid::new_v4(),
                name
            })),
            internal: Rc::new(Cell::new(0))
        }
    }
    pub fn make_addr(&self) -> EntAddr {
        let a: *mut Entity = self.data;
        let b = unsafe { &mut *a };

        EntAddr {
            data: b,
            internal: Rc::downgrade(&self.internal)
        }
    }
}

impl Drop for EntityHolder {
    fn drop(&mut self) {
        unsafe { Box::from_raw(self.data) };
        if std::thread::panicking() { return; }
        assert!(self.internal.get() >= 0, "Element Holder dropped while a mutable reference is held");
        assert!(self.internal.get() <= 0, "Element Holder dropped while immutable references are held");
    }
}

#[derive(Clone)]
pub struct EntAddr {
    data: *mut Entity,
    internal: Weak<Cell<i64>>
}

impl PartialEq for EntAddr {
    fn eq(&self, other: &Self) -> bool {
        (self.data == other.data) || (!self.valid() && !other.valid())
    }
}

impl Eq for EntAddr { }

impl Hash for EntAddr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl Serialize for EntAddr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        // intentionally provoke a panic here if valid but get_ref fails
        match self.valid() {
            true => serializer.serialize_i64(self.get_ref().unwrap().get_id().as_u128() as i64),
            false => serializer.serialize_i64(0i64)
        }
    }
}

impl<'de> Deserialize<'de> for EntAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
    D: serde::Deserializer<'de> {
        let v: i64 = Deserialize::deserialize(deserializer)?;

        Ok(map_id(Uuid::from_u128(v as u128)))
    }
}

unsafe impl Send for EntAddr {}
unsafe impl Sync for EntAddr {}

impl EntAddr {
    pub fn new() -> Self {
        Self {
            data: std::ptr::null_mut(),
            internal: Weak::new()
        }
    }
    pub fn valid(&self) -> bool {
        self.internal.strong_count() > 0
    }
    pub fn get_ref(&self) -> Option<EntRef> {
        match self.internal.upgrade() {
            Some(rc) => {
                if rc.get() >= 0 {
                    return Some(EntRef::new(
                        unsafe { &*self.data },
                        self.internal.clone()
                    ))
                }
                None
            },
            None => None
        }
    }
    pub fn get_ref_mut(&self) -> Option<EntRefMut> {
        match self.internal.upgrade() {
            Some(rc) => {
                if rc.get() == 0 {
                    return Some(EntRefMut::new(
                        unsafe { &mut *self.data },
                        self.internal.clone()
                    ))
                }
                None
            },
            None => None
        }
    }
}

pub struct EntRef<'a> {
    data: &'a Entity,
    internal: Weak<Cell<i64>>
}

pub struct EntRefMut<'a> {
    data: &'a mut Entity,
    internal: Weak<Cell<i64>>
}

impl<'a> EntRef<'a> {
    pub fn new(data: &'a Entity, internal: Weak<Cell<i64>>) -> Self {
        if let Some(rc) = internal.upgrade() {
            assert!(rc.get() >= 0, "Instance of Entity is already borrowed mutably");

            rc.set(rc.get() + 1);
        } else {
            panic!("Immutable Reference to Entity attempted to be created from a dead address");
        }

        Self { data, internal }
    }
}
impl<'a> EntRefMut<'a> {
    pub fn new(data: &'a mut Entity, internal: Weak<Cell<i64>>) -> Self {
        if let Some(rc) = internal.upgrade() {
            assert!(rc.get() <= 0, "Instance of Entity is already borrowed immutably");
            assert!(rc.get() >= 0, "Instance of Entity is already borrowed mutably");

            rc.set(rc.get() - 1);
        } else {
            panic!("Mutable Reference to Entity attempted to be created from a dead address");
        }

        Self { data, internal }
    }
}
impl<'a> Drop for EntRef<'a> {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        let rc = match self.internal.upgrade() {
            Some(rc) => rc,
            None => panic!("When dropping immutable reference of Entity, the holder was already destroyed")
        };
        rc.set(rc.get() - 1);
        assert!(rc.get() >= 0, "Instance of Entity's ref count somehow dropped below zero");
    }
}
impl<'a> Drop for EntRefMut<'a> {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        let rc = match self.internal.upgrade() {
            Some(rc) => rc,
            None => panic!("When dropping mutable reference of Entity, the holder was already destroyed")
        };
        rc.set(rc.get() + 1);
        assert!(rc.get() == 0, "Instance of Entity's ref count didn't equal zero when dropping mutable reference");
    }
}
impl<'a> Deref for EntRef<'a> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a> Deref for EntRefMut<'a> {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a> DerefMut for EntRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

// Manager
pub struct Manager {
    entities: Vec<EntityHolder>,
    root_entities: Vec<EntAddr>,
    entity_destroy_queue: HashSet<EntAddr>,
    element_destroy_queue: HashSet<EleAddrErased>
}

impl Manager {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            root_entities: Vec::new(),
            entity_destroy_queue: HashSet::new(),
            element_destroy_queue: HashSet::new()
        }
    }
    
    // Creation and destruction functions
    pub fn create_entity(&mut self, name: String) ->    EntAddr {
        self.entities.push(EntityHolder::new(name));
        let res = self.entities.last_mut().unwrap().make_addr();
        self.root_entities.push(res.clone());
        res.get_ref_mut().expect("Entity that was just created should exist").self_addr = res.clone();
        res
    }
    pub fn destroy_entity(&mut self, addr: EntAddr) {
        self.entity_destroy_queue.insert(addr);
    }
    pub fn destroy_element(&mut self, addr: EleAddrErased) {
        self.element_destroy_queue.insert(addr);
    }
    
    // Manager activity functions
    pub fn resolve(&mut self) {
        {
            let mut tmp_destroy_queue = self.entity_destroy_queue.iter().map(|ent| ent.clone()).collect::<Vec<EntAddr>>();
            self.entity_destroy_queue.clear();

            while !tmp_destroy_queue.is_empty() {
                let destroying = tmp_destroy_queue[tmp_destroy_queue.len() - 1].clone();
                self.reparent(destroying.clone(), EntAddr::new()).unwrap();
                tmp_destroy_queue.remove(tmp_destroy_queue.len() - 1);
                let children = destroying.get_ref().unwrap().get_children();
                for child in children.into_iter() {
                    self.reparent(child.clone(), EntAddr::new()).unwrap();
                    assert!(child.valid());
                    tmp_destroy_queue.push(child);
                }
                let root_index = self.root_entities.iter().position(|ent| *ent == destroying).unwrap();
                self.root_entities.remove(root_index);
                let index = self.find_ent_index(&destroying).unwrap();
                self.entities.remove(index);
            }
        }

        {
            let cloned_destroy_queue = self.element_destroy_queue.clone();
            self.element_destroy_queue.clear();
            for to_destroy in cloned_destroy_queue.iter() {
                if let Some(destroy_index) = self.find_ent_index(&to_destroy.get_owner()) {
                    let addr = self.entities[destroy_index].make_addr();
                    let mut r = addr.get_ref_mut().unwrap();
                    let ent_raw = r.deref_mut();
                    ent_raw.remove_element(to_destroy.clone());
                }
            }
        }
    }
    pub fn update(&mut self) {
        let mut index = 0 as usize;
        while index < self.entities.len() {
            let ent_addr = self.entities[index].make_addr();

            {
                let mut element_index = 0 as usize;
                let mut elements = ent_addr.get_ref_mut().unwrap().erased_elements();
                while element_index < elements.len() {
                    elements[element_index].get_ref_mut().unwrap().update(self, ent_addr.clone());
                    element_index += 1;
                }
            }

            self.resolve();

            index += 1;
        }

        self.resolve();
    }
    
    // Querying functions
    pub fn of_type<T: Element>(&mut self) ->                    Vec<EleAddr<T>> {
        self.entities.iter_mut()
        .filter(|ent| ent.make_addr().get_ref_mut().unwrap().query_element_addr::<T>().valid() )
        .map(|ent| { ent.make_addr().get_ref_mut().unwrap().query_element_addr::<T>() })
        .collect()
    }
    pub fn all_entities(&self) ->                               Vec<EntAddr> {
        self.entities.iter().map(|holder| holder.make_addr()).collect()
    }
    pub fn root_entities(&self) ->                                  Vec<EntAddr> {
        self.root_entities.clone()
    }

    // Hierarchy
    // performs cycle check, doesn't reparent if a cycle would be formed
    pub fn reparent(&mut self, child: EntAddr, parent: EntAddr) ->  Result<(), EntReferenceCycleError> {
        {
            assert!(child.valid());

            let child_ref = child.get_ref().unwrap();

            if child_ref.parent_addr == parent {
                return Ok(());
            }

            let mut curr = parent.clone();
            while curr.valid() {
                if curr == child {
                    return Err(EntReferenceCycleError);
                } else {
                    let new_curr = curr.get_ref().unwrap().parent_addr.clone();
                    curr = new_curr;
                }
            }

            // this code is *bad*
            {
                let mut op_on_vec = |op: &dyn Fn(&mut Vec<EntAddr>) -> usize| {
                    match child_ref.parent_addr.valid() {
                        true => {
                            op(&mut child_ref.parent_addr
                            .get_ref_mut().unwrap()
                            .children_addrs)
                        },
                        false => op(&mut self.root_entities)
                    }
                };

                let index = op_on_vec(&|vec| vec.iter().position(|addr| *addr == child).unwrap());

                op_on_vec(&|vec| { vec.remove(index); 0 });
            }

            if parent.valid() {
                parent.get_ref_mut().unwrap().children_addrs.push(child.clone());
            } else {
                self.root_entities.push(child.clone());
            }
        }

        child.get_ref_mut().unwrap().parent_addr = parent;
        
        Ok(())
    }

    // Utility function
    fn find_ent_index(&mut self, addr: &EntAddr) ->     Option<usize> {
        self.entities.iter_mut().position(|ent| ent.make_addr().eq(addr))
    }
}