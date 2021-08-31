use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};
use std::cell::{Cell, RefCell};
use std::any::Any;
use std::hash::Hash;

use crate::entity::*;

// Utility functions
fn static_dyn_ref_null() -> &'static mut dyn Element {
    unsafe { std::mem::transmute([0, 0, 0, 0]) }
}
fn static_dyn_ref_from_concrete<T: Element>(concrete: &mut T) -> &'static mut dyn Element {
    unsafe { std::mem::transmute(concrete as &mut dyn Element) }
}

pub trait Element : 'static {
    fn update(&mut self, _man: &mut Manager, _owner: EntAddr) { }
    #[cfg(feature = "gen-imgui")]
    fn fill_ui(&mut self, ui: &mut imgui::Ui) {
        ui.text(imgui::im_str!("Unimplemented ui"));
    }
}

pub struct ElementHolder {
    data: Box<RefCell<dyn Any>>, // must be cleaned up with a Box::from_raw
    element_ptr: &'static mut dyn Element,
    internal: Rc<Cell<i64>>,
    id: std::any::TypeId,
    owner: EntAddr
}

impl ElementHolder {
    pub fn new<T: Element>(val: T, owner: EntAddr) -> Self {
        let mut res = Self {
            data: Box::new(RefCell::new(val)),
            element_ptr: static_dyn_ref_null(), // value overwritten later, just ignore and don't use for now 
            internal: Rc::new(Cell::new(0)),
            id: std::any::TypeId::of::<T>(),
            owner
        };
        res.element_ptr = static_dyn_ref_from_concrete(res.make_addr::<T>().get_ref_mut().unwrap().deref_mut());
        res
    }
    pub fn get_ent(&self) -> EntAddr {
        self.owner.clone()
    }
    pub fn get_id(&self) -> std::any::TypeId {
        self.id
    }
    pub fn get_dyn_ref(&self) -> &dyn Element {
        self.element_ptr
    }
    pub fn get_dyn_ref_mut(&mut self) -> &mut dyn Element {
        self.element_ptr
    }
    pub fn make_addr<T: Element>(&mut self) -> EleAddr<T> {
        let c = match (&mut *(self.data.get_mut())).downcast_mut::<T>() {
            Some(c) => c,
            None => return EleAddr::new()
        };

        EleAddr::<T> {
            data: c,
            internal: Rc::downgrade(&self.internal),
            owner: self.owner.clone()
        }
    }
    pub fn make_addr_erased(&mut self) -> EleAddrErased {
        EleAddrErased {
            data: self.get_dyn_ref_mut(),
            internal: Rc::downgrade(&self.internal),
            owner: self.owner.clone()
        }
    }
}

impl Drop for ElementHolder {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        assert!(self.internal.get() >= 0, "Element Holder dropped while a mutable reference is held");
        assert!(self.internal.get() <= 0, "Element Holder dropped while immutable references are held");
    }
}

// Element Ref
pub struct EleAddr<T: Element> {
    data: *mut T,
    internal: Weak<Cell<i64>>,
    owner: EntAddr
}
impl<T: Element> Clone for EleAddr<T> {
    fn clone(&self) -> Self {
        Self { data: self.data.clone(), internal: self.internal.clone(), owner: self.owner.clone() }
    }
}
impl<T: Element> EleAddr<T> {
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
    pub fn get_ref<'a>(&self) -> Option<EleRef<'a, T>> {
        match self.internal.upgrade() {
            Some(_) => {
                let d = unsafe { &*self.data };

                Some(EleRef::new(
                    unsafe { std::mem::transmute::<&T, &'a T>(d) }, // rewrite the lifetime
                    self.internal.clone()
                ))
            },
            None => None
        }
    }
    pub fn get_ref_mut<'a>(&mut self) -> Option<EleRefMut<'a, T>> {
        match self.internal.upgrade() {
            Some(_) => {
                let d = unsafe { &mut *self.data };
                
                Some(EleRefMut::new(
                    unsafe { std::mem::transmute::<&mut T, &'a mut T>(d) }, // rewrite the lifetime
                    self.internal.clone()
                ))
            },
            None => None
        }
    }
}
pub struct EleRef<'a, T: Element> {
    data: &'a T,
    internal: Weak<Cell<i64>>
}
pub struct EleRefMut<'a, T: Element> {
    pub data: &'a mut T,
    pub internal: Weak<Cell<i64>>
}

impl<'a, T: Element> Drop for EleRef<'a, T> {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        let rc = match self.internal.upgrade() {
            Some(rc) => rc,
            None => panic!("When dropping immutable reference of type \"{}\", the holder was already destroyed", std::any::type_name::<T>())
        };
        rc.set(rc.get() - 1);
        assert!(rc.get() >= 0, "Instance of Element \"{}\"'s ref count somehow dropped below zero", std::any::type_name::<T>());
    }
}
impl<'a, T: Element> Drop for EleRefMut<'a, T> {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        let rc = match self.internal.upgrade() {
            Some(rc) => rc,
            None => panic!("When dropping mutable reference of type \"{}\", the holder was already destroyed", std::any::type_name::<T>())
        };
        rc.set(rc.get() + 1);
        assert!(rc.get() == 0, "Instance of Element \"{}\"'s ref count didn't equal zero when dropping mutable reference", std::any::type_name::<T>());
    }
}
impl<'a, T: Element> Deref for EleRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a, T: Element> Deref for EleRefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a, T: Element> DerefMut for EleRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}
impl<'a, T: Element> EleRef<'a, T> {
    fn new(data: &'a T, internal: Weak<Cell<i64>>) -> Self {
        if let Some(rc) = internal.upgrade() {
            assert!(rc.get() >= 0, "Instance of Element \"{}\" is already borrowed mutably", std::any::type_name::<T>());

            rc.set(rc.get() + 1);
        } else {
            panic!("Immutable Reference to element \"{}\" attempted to be created from a dead address", std::any::type_name::<T>());
        }

        Self { data, internal }
    }
}
impl<'a, T: Element> EleRefMut<'a, T> {
    pub fn new(data: &'a mut T, internal: Weak<Cell<i64>>) -> Self {
        if let Some(rc) = internal.upgrade() {
            assert!(rc.get() <= 0, "Instance of Element \"{}\" is already borrowed immutably", std::any::type_name::<T>());
            assert!(rc.get() >= 0, "Instance of Element \"{}\" is already borrowed mutably",   std::any::type_name::<T>());

            rc.set(rc.get() - 1);
        } else {
            panic!("Mutable Reference to element \"{}\" attempted to be created from a dead address", std::any::type_name::<T>());
        }

        Self { data, internal }
    }
}

// Element Ref Erased
#[derive(Clone)]
pub struct EleAddrErased {
    data: *mut dyn Element,
    internal: Weak<Cell<i64>>,
    owner: EntAddr
}

impl EleAddrErased {
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
    pub fn get_ref<'a>(&self) -> Option<EleRefErased<'a>> {
        match self.internal.upgrade() {
            Some(_) => {
                let d = unsafe { &*self.data };

                Some(EleRefErased::new(
                    unsafe { std::mem::transmute::<&dyn Element, &'a dyn Element>(d) }, // rewrite the lifetime
                    self.internal.clone()
                ))
            },
            None => None
        }
    }
    pub fn get_ref_mut<'a>(&mut self) -> Option<EleRefErasedMut<'a>> {
        match self.internal.upgrade() {
            Some(_) => {
                let d = unsafe { &mut *self.data };
                
                Some(EleRefErasedMut::new(
                    unsafe { std::mem::transmute::<&mut dyn Element, &'a mut dyn Element>(d) }, // rewrite the lifetime
                    self.internal.clone()
                ))
            },
            None => None
        }
    }
    pub fn get_owner(&self) -> EntAddr {
        self.owner.clone()
    }
}

pub struct EleRefErased<'a> {
    data: &'a dyn Element,
    internal: Weak<Cell<i64>>
}
pub struct EleRefErasedMut<'a> {
    pub data: &'a mut dyn Element,
    pub internal: Weak<Cell<i64>>
}

impl<'a> Drop for EleRefErased<'a> {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        let rc = match self.internal.upgrade() {
            Some(rc) => rc,
            None => panic!("When dropping immutable reference of type Compononet Erased, the holder was already destroyed")
        };
        rc.set(rc.get() - 1);
        assert!(rc.get() >= 0, "Instance of Element Erased's ref count somehow dropped below zero");
    }
}
impl<'a> Drop for EleRefErasedMut<'a> {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        let rc = match self.internal.upgrade() {
            Some(rc) => rc,
            None => panic!("When dropping mutable reference of type Element Erased, the holder was already destroyed")
        };
        rc.set(rc.get() + 1);
        assert!(rc.get() == 0, "Instance of Element Erased's ref count didn't equal zero when dropping mutable reference");
    }
}
impl<'a> Deref for EleRefErased<'a> {
    type Target = dyn Element;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a> Deref for EleRefErasedMut<'a> {
    type Target = dyn Element;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a> DerefMut for EleRefErasedMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}
impl<'a> EleRefErased<'a> {
    fn new(data: &'a dyn Element, internal: Weak<Cell<i64>>) -> Self {
        if let Some(rc) = internal.upgrade() {
            assert!(rc.get() >= 0, "Instance of Element is already borrowed mutably");

            rc.set(rc.get() + 1);
        } else {
            panic!("Immutable Reference to Element Erased attempted to be created from a dead address");
        }

        Self { data, internal }
    }
}
impl<'a> EleRefErasedMut<'a> {
    pub fn new(data: &'a mut dyn Element, internal: Weak<Cell<i64>>) -> Self {
        if let Some(rc) = internal.upgrade() {
            assert!(rc.get() <= 0, "Instance of Element Erased is already borrowed immutably");
            assert!(rc.get() >= 0, "Instance of Element Erased is already borrowed mutably");

            rc.set(rc.get() - 1);
        } else {
            panic!("Mutable Reference to Element Erased attempted to be created from a dead address");
        }

        Self { data, internal }
    }
}

impl<T: Element> From<EleAddr<T>> for EleAddrErased {
    fn from(other: EleAddr<T>) -> Self {
        match other.valid() {
            true => {
                let mut other_mut = other.clone();
                EleAddrErased {
                    data: static_dyn_ref_from_concrete(other_mut.get_ref_mut().unwrap().deref_mut()),
                    internal: other.internal.clone(),
                    owner: other.owner.clone()
                }
            },
            false => EleAddrErased::new()
        }
    }
}

impl PartialEq for EleAddrErased {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl Eq for EleAddrErased { }

impl Hash for EleAddrErased {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}