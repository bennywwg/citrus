use std::{any::{Any, TypeId}, collections::HashMap, fmt::Debug, rc::Rc};
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

#[derive(Debug)]
pub enum SceneSerdeError {
    InternalError(String),
    SerdeError(Vec<serde_json::Error>)
}

impl From<serde_json::Error> for SceneSerdeError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerdeError(vec!(err))
    }
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
    
    pub fn deserialize_empty_into(&self, ent: EntAddr, name: String) -> Result<EleAddrErased, SceneSerdeError> {
        // find creator
        let entry =
        self.find_exact_creator(name.as_str())
        .ok_or(SceneSerdeError::InternalError(format!("Element {} not registered", name)))?;


        {
            let cloned = ent.clone();
            let mut ent_ref = cloned.get_ref_mut().unwrap();
            assert!(!ent_ref.query_element_addr_by_id(&entry.id).valid());
        }

        let erased = (entry.creator)(ent);

        assert!(erased.valid());

        Ok(erased)
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

        let payload = ele.get_ref().unwrap().ecs_serialize();

        Some(serde_json::to_value(ElementObj {
            name: creator.name,
            payload
        }).unwrap())
    }
    pub fn deserialize_scene(&mut self, man: &mut Manager, content: serde_json::Value) -> Option<Vec<EntAddr>> {
        #[derive(Deserialize, Clone)]
        struct EleObj {
            name: String,
            payload: serde_json::Value
        }

        #[derive(Deserialize, Clone)]
        struct EntObj {
            name: String,
            id: i64,
            eles: Vec<EleObj>
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

        // First create empty elements in their respective entities so no EleAddr deserialize
        // fails due to the element not yet being added
        
        struct EleAddrDeserializeState {
            ele: EleAddrErased,
            payload: serde_json::Value
        }

        let refs =
        pairs.iter().map(|pair| {
            pair.0.eles.iter().map(|ele_obj| {
                self.deserialize_empty_into(pair.1.clone(), ele_obj.name.clone())
                .map(|ele| EleAddrDeserializeState {
                    ele,
                    payload: ele_obj.payload.clone()
                })
            })
            .filter(|res| res.is_ok() )
            .map(|res| res.ok().unwrap() )
            .collect::<Vec<EleAddrDeserializeState>>()
        })
        .flatten()
        .collect::<Vec<EleAddrDeserializeState>>();

        refs.iter().for_each(|ele_state| {
            ele_state.ele.clone().get_ref_mut().unwrap().ecs_deserialize(ele_state.payload.clone()).unwrap();
        });

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
                let erased_eles=
                ea.get_ref_mut().unwrap()
                .erased_elements();
                let eles: Vec<serde_json::Value>=
                    erased_eles
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