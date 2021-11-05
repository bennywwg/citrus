pub mod element;
pub mod entity;
pub mod deserialize_context;
pub mod reflection;

#[macro_use]
extern crate lazy_static;

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;


    use serde::*;
    use crate::element::*;
    use crate::entity::*;

    #[derive(Clone, Serialize, Deserialize)]
    pub struct PosRot {
        pos: [f32; 3]
    }

    impl Element for PosRot { }

    #[derive(Clone, Serialize)]
    pub struct Mesh {
        pub pos: EleAddr<PosRot>
    }

    impl Element for Mesh {
        fn update(&mut self, _man: &mut Manager, _owner: EntAddr) {
            if let Some(mut pos_ref) = self.pos.get_ref_mut() {
                (*pos_ref).pos[1] = (*pos_ref).pos[1] + 1.0;
                (*pos_ref).pos[2] = (*pos_ref).pos[1] + 3.0;
            }
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    struct A {
        val: i32
    }

    impl Element for A {
        fn update(&mut self, _man: &mut Manager, _owner: EntAddr) {
            println!("A: val = {}", self.val);
            self.val += 10;
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    struct B {
        bal: i32
    }

    impl Element for B { }

    #[test]
    fn test_ecs() {
        let eh = EntityHolder::new("test entity".to_string());
        let mut er = eh.make_addr();
        assert!(er.valid());
    
        {
            assert!(if let Some(_) = er.get_ref() { true } else { false });
            assert!(if let Some(_) = er.get_ref_mut() { true } else { false });
            assert!(er.valid());
        }
    
        let mut e = er.get_ref_mut().expect("Entity should exist");
    
        let mut c = e.add_element(A { val: 10 }).expect("Expected element addr to be returned after adding");
        assert!(c.valid());
        
        assert!(e.query_element_addr::<A>().valid());
        assert!(!e.query_element_addr::<B>().valid());
    
        // address
        {
            assert!(c.get_ref().expect("Expect Element to exist").val == 10);
            c.get_ref_mut().expect("Expect Element to exist").val = 20;
            assert!(c.get_ref().expect("Expect Element to exist").val == 20);
            assert!(c.valid());
        }
    
        e.remove_element::<A>().expect("Expected to remvoe element normally");
        assert!(!e.query_element_addr::<A>().valid());
    
        // address
        {
            assert!(if let None = c.get_ref() { true } else { false });
            assert!(if let None = c.get_ref_mut() { true } else { false });
            assert!(!c.valid());
        }
    }
    
    #[test]
    fn test_manager_query() {
        let mut m = Manager::new();
        let mut e = m.create_entity("test entity".to_string());
        e.get_ref_mut().unwrap().add_element(A { val: 1 }).expect("Expected to add element successfully");

        assert!(m.of_type::<A>().len() == 1);
    }
}