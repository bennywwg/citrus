use std::{any::{Any, TypeId}, collections::HashMap, rc::Rc};
use serde::*;
use uuid::Uuid;

use crate::element::*;
use crate::entity::*;
use crate::deserialize_context::*;

#[derive(Clone)]
pub struct CreatorEntry {
    pub creator: Rc<Box<dyn Fn(EntAddr) -> EleAddrErased>>,
    pub name: String,
    pub id: TypeId
}

pub struct SceneSerde {
    creator_map: HashMap<TypeId, CreatorEntry>
}

impl SceneSerde {
    pub fn new() -> Self {
        Self {
            creator_map: HashMap::new()
        }
    }
    pub fn register_element_creator<T: Element + Any + Clone>(&mut self, default: T, name: &str) {
        let id = TypeId::of::<T>();
        self.creator_map.insert(id, CreatorEntry {
            creator: Rc::new(Box::new(move |ent| {
                match ent.clone().get_ref_mut() {
                    Some(mut e) => {
                        e
                        .add_element(default.clone())
                        .map_or(EleAddrErased::new(), |a| a.into())
                    },
                    None => {
                        EleAddrErased::new()
                    }
                }
            })),
            name: name.into(),
            id: std::any::TypeId::of::<T>()
        });
    }

    // serde
    pub fn deserialize_element_into(&mut self, ent: EntAddr, val: serde_json::Value) -> EleAddrErased {
        #[derive(Deserialize)]
        struct ElementObj {
            name: String,
            payload: serde_json::Value
        }

        let create_data = match serde_json::from_value::<ElementObj>(val) {
            Ok(data) => data,
            Err(_) => return EleAddrErased::new()
        };

        let entry = match self.find_exact_creator(create_data.name.as_str()) {
            Some(entry) => entry,
            None => return EleAddrErased::new()
        };

        let mut erased = (entry.creator)(ent);

        assert!(erased.valid());

        erased.get_ref_mut().unwrap().ecs_deserialize(create_data.payload);

        erased
    }
    pub fn serialize_element(&mut self, ele: EleAddrErased) -> Option<serde_json::Value> {
        #[derive(Serialize)]
        struct ElementObj {
            name: String,
            payload: serde_json::Value
        }

        if !ele.valid() {
            return None;
        }

        let creator = self.find_exact_creator_by_id(ele.get_element_type_id().unwrap()).unwrap();

        Some(serde_json::to_value(ElementObj {
            name: creator.name,
            payload: ele.get_ref().unwrap().ecs_serialize()
        }).unwrap())
    }
    pub fn deserialize_scene(&mut self, man: &mut Manager, content: serde_json::Value) -> Option<Vec<EntAddr>> {
        #[derive(Deserialize, Clone)]
        struct EntObj {
            name: String,
            id: i64,
            eles: Vec<serde_json::Value>
        }

        let r = serde_json::from_value::<Vec<EntObj>>(content);
        let ent_objs = match r {
            Ok(v) => v,
            Err(er) => {
                println!("{:?}", er);
                panic!();
            }
        };
        
        begin_deserialize();

        let mut pairs = Vec::<(EntObj, EntAddr)>::new();

        ent_objs
        .iter()
        .for_each(|ent| pairs.push((ent.clone(), set_mapping(Uuid::from_u128(ent.id as u128), ent.name.clone(), man))));

        for pair in pairs.iter() {
            for ele_payload in pair.0.eles.iter() {
                if !self.deserialize_element_into(pair.1.clone(), ele_payload.clone()).valid() {
                    println!("Deserializing element failed");
                }
            }
        }

        end_deserialize();

        Some(pairs.iter().map(|pair| pair.1.clone()).collect())
    }
    pub fn serialize_scene(&mut self, _man: &mut Manager, content: Vec<EntAddr>) -> serde_json::Value {
        #[derive(Serialize)]
        struct EntObj {
            name: String,
            id: i64,
            eles: Vec<serde_json::Value>
        }

        let ent_objs: Vec<EntObj>
            =content
            .iter()
            .map(|ea| {
                let eles: Vec<serde_json::Value>
                    =ea.get_ref_mut().unwrap()
                    .erased_elements()
                    .iter().filter_map(|ele| {
                        self.serialize_element(ele.clone())
                    })
                    .collect();

                EntObj {
                    name: ea.get_ref().unwrap().name.clone(),
                    id: match ea.get_ref() {
                        None => 0i64,
                        Some(e) => e.get_id().as_u128() as i64
                    },
                    eles
                }
            })
            .collect();
        
        serde_json::to_value(ent_objs).unwrap()
    }   

    // Utility functions
    pub fn find_creators(&self, name: &str) -> Vec<CreatorEntry> {
        let mut res = Vec::<CreatorEntry>::new();
        for b in self.creator_map.iter() {
            if b.1.name.contains(name) {
                res.push(b.1.clone());
            }
        }
        res
    }
    pub fn find_exact_creator(&self, name: &str) -> Option<CreatorEntry> {
        for b in self.creator_map.iter() {
            if b.1.name == name {
                return Some(b.1.clone());
            }
        }
        None
    }
    pub fn find_exact_creator_by_id(&self, id: TypeId) -> Option<CreatorEntry> {
        for b in self.creator_map.iter() {
            if b.1.id == id {
                return Some(b.1.clone());
            }
        }
        None
    }
}