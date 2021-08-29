pub mod element;
pub mod entity;

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::element::*;
    use crate::entity::*;

    #[derive(Clone)]
    pub struct PosRot {
        pos: [f32; 3]
    }

    impl Element for PosRot { }

    #[derive(Clone)]
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

    #[derive(Clone)]
    struct A {
        val: i32
    }

    impl Element for A {
        fn update(&mut self, _man: &mut Manager, _owner: EntAddr) {
            println!("A: val = {}", self.val);
            self.val += 10;
        }
    }

    impl Drop for A {
        fn drop(&mut self) {
            println!("A dropped");
        }
    }

    #[derive(Clone)]
    struct B {
        bal: i32
    }

    impl Element for B { }

    #[test]
    fn test_ecs() {
        let eh = EntityHolder::new();
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
    fn test_ecs_manager() {
        struct DropTest {
            val: Rc<Cell<i32>>
        }

        impl Element for DropTest { }
        impl Drop for DropTest {
            fn drop(&mut self) {
                (*self.val).set(1);
            }
        }

        let val = Rc::new(Cell::new(0));


        let mut m = Manager::new();
        let mut e = m.create_entity();
        let ca = e.get_ref_mut().unwrap().add_element(DropTest { val: val.clone() }).expect("Expected to add element successfully");

        assert!((*val).get() == 0);
        m.destroy_element(ca.into());
        m.update();
        assert!((*val).get() == 1);
        
        m.destroy_entity(e);
        m.update();
    }

    #[test]
    fn test_manager_query() {
        let mut m = Manager::new();
        let mut e = m.create_entity();
        let ca = e.get_ref_mut().unwrap().add_element(A { val: 1 }).expect("Expected to add element successfully");

        assert!(m.of_type::<A>().len() == 1);
    }
}