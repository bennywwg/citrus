use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};
use std::cell::{Cell};
use std::any::Any;

use crate::entity::*;

pub trait Component : 'static {
    fn on_attach_to(&self, ent: &Entity);
    fn on_update(&self);
    fn on_detach(&self);
    fn clone_component(&self) -> ComponentHolder;
}

pub struct ComponentHolder {
    data: *mut dyn Any, // must be cleaned up with a Box::from_raw
    internal: Rc<Cell<i64>>,
    id: std::any::TypeId
}

impl ComponentHolder {
    pub fn new<T: Component>(val: T) -> Self {
        Self {
            data: Box::into_raw(Box::new(val)),
            internal: Rc::new(Cell::new(0)),
            id: std::any::TypeId::of::<T>()
        }
    }
    pub fn get_id(&self) -> std::any::TypeId {
        self.id
    }
    pub fn make_addr<T: Component>(&mut self) -> Option<ComponentAddr<T>> {
        //let mut extended_val = unsafe { std::mem::transmute::<&mut ComponentHolder, &'static mut ComponentHolder>(val) };
        //let mut extended_data = extended_val.data;
        let a: *mut dyn Any = self.data;
        let b = unsafe { &mut *a };
        let c = b.downcast_mut::<T>()?;

        Some(ComponentAddr::<T> {
            //data: unsafe { std::mem::transmute::<&mut T, &'static mut T>(c) }, // rewrite the lifetime
            data: c,
            internal: Rc::downgrade(&self.internal)
        })
    }
}

impl Drop for ComponentHolder {
    fn drop(&mut self) {
        unsafe { Box::from_raw(self.data) };
        if std::thread::panicking() { return; }
        assert!(self.internal.get() >= 0, "Component Holder dropped while a mutable reference is held");
        assert!(self.internal.get() <= 0, "Component Holder dropped while immutable references are held");
    }
}

#[derive(Clone)]
pub struct ComponentAddr<T: Component> {
    data: *mut T,
    internal: Weak<Cell<i64>>
}

impl<T: Component> ComponentAddr<T> {
    pub fn new() -> Self {
        Self {
            data: std::ptr::null_mut(),
            internal: Weak::new()
        }
    }
    pub fn valid(&self) -> bool {
        self.internal.strong_count() > 0
    }
    pub fn get_ref<'a>(&self) -> Option<CptRef<'a, T>> {
        match self.internal.upgrade() {
            Some(_) => {
                let d = unsafe { &*self.data };

                Some(CptRef::new(
                    unsafe { std::mem::transmute::<&T, &'a T>(d) }, // rewrite the lifetime
                    self.internal.clone()
                ))
            },
            None => None
        }
    }
    pub fn get_ref_mut<'a>(&mut self) -> Option<CptRefMut<'a, T>> {
        match self.internal.upgrade() {
            Some(_) => {
                let d = unsafe { &mut *self.data };
                
                Some(CptRefMut::new(
                    unsafe { std::mem::transmute::<&mut T, &'a mut T>(d) }, // rewrite the lifetime
                    self.internal.clone()
                ))
            },
            None => None
        }
    }
}

pub struct CptRef<'a, T: Component> {
    data: &'a T,
    internal: Weak<Cell<i64>>
}
pub struct CptRefMut<'a, T: Component> {
    pub data: &'a mut T,
    pub internal: Weak<Cell<i64>>
}

impl<'a, T: Component> Drop for CptRef<'a, T> {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        let rc = match self.internal.upgrade() {
            Some(rc) => rc,
            None => panic!("When dropping immutable reference of type \"{}\", the holder was already destroyed", std::any::type_name::<T>())
        };
        rc.set(rc.get() - 1);
        assert!(rc.get() >= 0, "Instance of Component \"{}\"'s ref count somehow dropped below zero", std::any::type_name::<T>());
    }
}
impl<'a, T: Component> Drop for CptRefMut<'a, T> {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        let rc = match self.internal.upgrade() {
            Some(rc) => rc,
            None => panic!("When dropping mutable reference of type \"{}\", the holder was already destroyed", std::any::type_name::<T>())
        };
        rc.set(rc.get() + 1);
        assert!(rc.get() == 0, "Instance of Component \"{}\"'s ref count didn't equal zero when dropping mutable reference", std::any::type_name::<T>());
    }
}
impl<'a, T: Component> Deref for CptRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a, T: Component> Deref for CptRefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a, T: Component> DerefMut for CptRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
impl<'a, T: Component> CptRef<'a, T> {
    fn new(data: &'a T, internal: Weak<Cell<i64>>) -> Self {
        if let Some(rc) = internal.upgrade() {
            assert!(rc.get() >= 0, "Instance of Component \"{}\" is already borrowed mutably", std::any::type_name::<T>());

            rc.set(rc.get() + 1);
        } else {
            panic!("Immutable Reference to component \"{}\" attempted to be created from a dead address", std::any::type_name::<T>());
        }

        Self { data, internal }
    }
}
impl<'a, T: Component> CptRefMut<'a, T> {
    pub fn new(data: &'a mut T, internal: Weak<Cell<i64>>) -> Self {
        if let Some(rc) = internal.upgrade() {
            assert!(rc.get() <= 0, "Instance of Component \"{}\" is already borrowed immutably", std::any::type_name::<T>());
            assert!(rc.get() >= 0, "Instance of Component \"{}\" is already borrowed mutably",   std::any::type_name::<T>());

            rc.set(rc.get() - 1);
        } else {
            panic!("Mutable Reference to component \"{}\" attempted to be created from a dead address", std::any::type_name::<T>());
        }

        Self { data, internal }
    }
}