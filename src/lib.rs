pub mod component;
pub mod entity;
pub mod manager;

#[cfg(test)]
mod tests {
    use crate::component::*;
    use crate::entity::*;
    use crate::manager::*;

    #[derive(Clone)]
    pub struct PosRot {
        pos: [f32; 3]
    }

    impl Component for PosRot { }

    #[derive(Clone)]
    struct A {
        val: i32
    }

    impl Component for A {
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

    impl Component for B { }

    #[test]
    fn test_ecs() {
        let mut eh = EntityHolder::new();
        let mut er = eh.make_addr();
        assert!(er.valid());
    
        {
            assert!(if let Some(_) = er.get_ref() { true } else { false });
            assert!(if let Some(_) = er.get_ref_mut() { true } else { false });
            assert!(er.valid());
        }
    
        let mut e = er.get_ref_mut().expect("Entity should exist");
    
        let mut c = e.add_component(A { val: 10 }).expect("Expected component addr to be returned after adding");
        assert!(c.valid());
        
        assert!(e.query_component_addr::<A>().valid());
        assert!(!e.query_component_addr::<B>().valid());
    
        // address
        {
            assert!(c.get_ref().expect("Expect Component to exist").val == 10);
            c.get_ref_mut().expect("Expect Component to exist").val = 20;
            assert!(c.get_ref().expect("Expect Component to exist").val == 20);
            assert!(c.valid());
        }
    
        e.remove_component::<A>().expect("Expected to remvoe component normally");
        assert!(!e.query_component_addr::<A>().valid());
    
        // address
        {
            assert!(if let None = c.get_ref() { true } else { false });
            assert!(if let None = c.get_ref_mut() { true } else { false });
            assert!(!c.valid());
        }
    }
    
    #[test]
    fn test_ecs_manager() {
        let mut m = Manager::new();
        let mut e = m.create_entity();
        e.get_ref_mut().unwrap().add_component(A { val: -50 }).expect("Expected to add component successfully");
        m.update();
        m.destroy_entity(e);
        m.update();
    }   
}