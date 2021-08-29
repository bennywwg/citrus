use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};
use std::cell::{Cell};
use std::any::Any;

use crate::{entity::EntAddr, manager::Manager};

pub trait Component : 'static {
    fn update(&mut self, _man: &mut Manager, _owner: EntAddr) { }
}

pub struct ComponentHolder {
    data: *mut dyn Any, // must be cleaned up with a Box::from_raw
    component_ptr: &'static mut dyn Component,
    internal: Rc<Cell<i64>>,
    id: std::any::TypeId,
    owner: EntAddr
}

impl ComponentHolder {
    pub fn new<T: Component>(val: T, owner: EntAddr) -> Self {
        let mut res = Self {
            data: Box::into_raw(Box::new(val)),
            component_ptr: unsafe { std::mem::transmute([1,2,3,4]) }, // value overwritten later, just ignore and don't use for now 
            internal: Rc::new(Cell::new(0)),
            id: std::any::TypeId::of::<T>(),
            owner
        };
        let mr = unsafe {
            std::mem::transmute::<&mut dyn Component, &'static mut dyn Component>(res.make_addr::<T>().get_ref_mut().unwrap().deref_mut())
        };
        let v = mr as &mut dyn Component;
        res.component_ptr = v;
        res
    }
    pub fn get_ent(&self) -> EntAddr {
        self.owner.clone()
    }
    pub fn get_id(&self) -> std::any::TypeId {
        self.id
    }
    pub fn get_dyn_ref_mut(&mut self) -> &mut dyn Component {
        self.component_ptr
    }
    pub fn make_addr<T: Component>(&mut self) -> ComponentAddr<T> {
        let a: *mut dyn Any = self.data;
        let b = unsafe { &mut *a };
        let c = match b.downcast_mut::<T>() {
            Some(c) => c,
            None => return ComponentAddr::new()
        };

        ComponentAddr::<T> {
            data: c,
            internal: Rc::downgrade(&self.internal),
            owner: self.owner.clone()
        }
    }
    pub fn make_addr_erased(&mut self) -> ComponentAddrErased {
        ComponentAddrErased {
            data: self.get_dyn_ref_mut(),
            internal: Rc::downgrade(&self.internal),
            owner: self.owner.clone()
        }
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

// Component Ref
#[derive(Clone)]
pub struct ComponentAddr<T: Component> {
    data: *mut T,
    internal: Weak<Cell<i64>>,
    owner: EntAddr
}
impl<T: Component> ComponentAddr<T> {
    pub fn new() -> Self {
        Self {
            data: std::ptr::null_mut(),
            internal: Weak::new(),
            owner: EntAddr::new()
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
        self.data
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

// Component Ref Erased
#[derive(Clone)]
pub struct ComponentAddrErased {
    data: *mut dyn Component,
    internal: Weak<Cell<i64>>,
    owner: EntAddr
}

impl ComponentAddrErased {
    pub fn new() -> Self {
        Self {
            data: unsafe { std::mem::transmute([0, 0, 0, 0]) },
            internal: Weak::new(),
            owner: EntAddr::new()
        }
    }
    pub fn valid(&self) -> bool {
        self.internal.strong_count() > 0
    }
    pub fn get_ref<'a>(&self) -> Option<CptRefErased<'a>> {
        match self.internal.upgrade() {
            Some(_) => {
                let d = unsafe { &*self.data };

                Some(CptRefErased::new(
                    unsafe { std::mem::transmute::<&dyn Component, &'a dyn Component>(d) }, // rewrite the lifetime
                    self.internal.clone()
                ))
            },
            None => None
        }
    }
    pub fn get_ref_mut<'a>(&mut self) -> Option<CptRefErasedMut<'a>> {
        match self.internal.upgrade() {
            Some(_) => {
                let d = unsafe { &mut *self.data };
                
                Some(CptRefErasedMut::new(
                    unsafe { std::mem::transmute::<&mut dyn Component, &'a mut dyn Component>(d) }, // rewrite the lifetime
                    self.internal.clone()
                ))
            },
            None => None
        }
    }
}

pub struct CptRefErased<'a> {
    data: &'a dyn Component,
    internal: Weak<Cell<i64>>
}
pub struct CptRefErasedMut<'a> {
    pub data: &'a mut dyn Component,
    pub internal: Weak<Cell<i64>>
}

impl<'a> Drop for CptRefErased<'a> {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        let rc = match self.internal.upgrade() {
            Some(rc) => rc,
            None => panic!("When dropping immutable reference of type Compononet Erased, the holder was already destroyed")
        };
        rc.set(rc.get() - 1);
        assert!(rc.get() >= 0, "Instance of Component Erased's ref count somehow dropped below zero");
    }
}
impl<'a> Drop for CptRefErasedMut<'a> {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        let rc = match self.internal.upgrade() {
            Some(rc) => rc,
            None => panic!("When dropping mutable reference of type Component Erased, the holder was already destroyed")
        };
        rc.set(rc.get() + 1);
        assert!(rc.get() == 0, "Instance of Component Erased's ref count didn't equal zero when dropping mutable reference");
    }
}
impl<'a> Deref for CptRefErased<'a> {
    type Target = dyn Component;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a> Deref for CptRefErasedMut<'a> {
    type Target = dyn Component;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a> DerefMut for CptRefErasedMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}
impl<'a> CptRefErased<'a> {
    fn new(data: &'a dyn Component, internal: Weak<Cell<i64>>) -> Self {
        if let Some(rc) = internal.upgrade() {
            assert!(rc.get() >= 0, "Instance of Component is already borrowed mutably");

            rc.set(rc.get() + 1);
        } else {
            panic!("Immutable Reference to Component Erased attempted to be created from a dead address");
        }

        Self { data, internal }
    }
}
impl<'a> CptRefErasedMut<'a> {
    pub fn new(data: &'a mut dyn Component, internal: Weak<Cell<i64>>) -> Self {
        if let Some(rc) = internal.upgrade() {
            assert!(rc.get() <= 0, "Instance of Component Erased is already borrowed immutably");
            assert!(rc.get() >= 0, "Instance of Component Erased is already borrowed mutably");

            rc.set(rc.get() - 1);
        } else {
            panic!("Mutable Reference to Component Erased attempted to be created from a dead address");
        }

        Self { data, internal }
    }
}