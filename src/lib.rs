pub mod component;
pub mod entity;
pub mod manager;

#[cfg(test)]
mod tests {
    use crate::component::*;
    use crate::entity::*;
    use crate::manager::*;

    struct A {
        val: i32
    }

    impl Component for A {
        fn on_attach_to(&self, _ent: &Entity) {
            todo!()
        }

        fn on_update(&self) {
            todo!()
        }

        fn on_detach(&self) {
            todo!()
        }

        fn clone_component(&self) -> ComponentHolder {
            ComponentHolder::new(Self {
                val: self.val
            })
        }
    }

    struct B {
        bal: i32
    }

    impl Component for B {
        fn on_attach_to(&self, _ent: &Entity) {
            todo!()
        }

        fn on_update(&self) {
            todo!()
        }

        fn on_detach(&self) {
            todo!()
        }

        fn clone_component(&self) -> ComponentHolder {
            ComponentHolder::new(Self {
                bal: self.bal
            })
        }
    }

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
        m.create_entity();
    }   
}