use std::{cell::Cell, ops::{Deref, DerefMut}, rc::{Rc, Weak}};

use crate::component::*;

pub struct Entity {
    components: Vec<ComponentHolder>
}

impl Entity {
    pub fn add_component<T: Component>(&mut self, val: T) -> Result<ComponentAddr<T>, String> {
        if self.query_component_addr::<T>().valid() {
            return Err(format!("Component of type \"{}\" is already present", std::any::type_name::<T>()));
        }
        self.components.push(ComponentHolder::new(val));
        Ok(self.query_component_addr::<T>())
    }
    pub fn remove_component<T: Component>(&mut self) -> Result<(), String> {
        if let Some(index) = self.components.iter_mut().position(|comp| comp.get_id() == std::any::TypeId::of::<T>()) {
            self.components.remove(index);
            Ok(())
        } else {
            Err(format!("Component \"{}\" did not exist", std::any::type_name::<T>()) as String)
        }
    }
    pub fn query_component_addr<T: Component>(&mut self) -> ComponentAddr<T> {
        for comp in self.components.iter_mut() {
            if comp.get_id() == std::any::TypeId::of::<T>() {
                return comp.make_addr::<T>().expect("Component ID matched but creating Addr failed");
            }
        }
        ComponentAddr::new()
    }
    pub fn query_component<T: Component>(&mut self) -> Option<CptRef<T>> {
        self.query_component_addr().get_ref()
    }
    pub fn query_component_mut<T: Component>(&mut self) -> Option<CptRefMut<T>> {
        self.query_component_addr().get_ref_mut()
    }
}

pub struct EntityHolder {
    data: *mut Entity, // must be cleaned up with a Box::from_raw
    internal: Rc<Cell<i64>>
}

impl EntityHolder {
    pub fn new() -> Self {
        Self {
            data: Box::into_raw(Box::new(Entity {
                components: Vec::new()
            })),
            internal: Rc::new(Cell::new(0))
        }
    }
    pub fn make_addr(&mut self) -> EntAddr {
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
        assert!(self.internal.get() >= 0, "Component Holder dropped while a mutable reference is held");
        assert!(self.internal.get() <= 0, "Component Holder dropped while immutable references are held");
    }
}

#[derive(Clone)]
pub struct EntAddr {
    data: *mut Entity,
    internal: Weak<Cell<i64>>
}

impl EntAddr {
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
    pub fn get_ref_mut(&mut self) -> Option<EntRefMut> {
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