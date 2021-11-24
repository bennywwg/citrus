use std::{any::{Any, TypeId}, collections::HashMap, fmt::Debug, rc::Rc};
use serde::*;
use uuid::Uuid;
use std::fmt;

use crate::element::*;
use crate::entity::*;
use crate::deserialize_context::*;

#[derive(Debug)]
pub enum SceneSerdeError {
    CycleError(String),
    MissingElementError(String),
    SerdeError(serde_json::Error)
}

impl fmt::Display for SceneSerdeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SceneSerdeError::CycleError(info) => write!(f, "{}", info),
            SceneSerdeError::MissingElementError(info) => write!(f, "{}", info),
            SceneSerdeError::SerdeError(err) => write!(f, "{}", err)
        }
    }
}

#[derive(Clone)]
pub struct CreatorEntry {
    pub creator: Rc<Box<dyn Fn(EntAddr) -> EleAddrErased>>,
    pub name: String,
    pub id: TypeId
}

pub struct SceneDeserResult {
    pub ents: Vec<EntAddr>,
    pub errors: Vec<SceneSerdeError>
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
    
    pub fn deserialize_empty_into(&self, ent: EntAddr, name: String) -> Result<EleAddrErased, SceneSerdeError> {
        // find creator
        let entry =
        self.find_exact_creator(name.as_str())
        .ok_or(SceneSerdeError::MissingElementError(name))?;

        {
            let cloned = ent.clone();
            let mut ent_ref = cloned.get_ref_mut().unwrap();
            assert!(!ent_ref.query_element_addr_by_id(&entry.id).valid());
        }

        let erased = (entry.creator)(ent);

        assert!(erased.valid());

        Ok(erased)
    }

    // Returns a Some(value) if ele is valid, otherwise returns None
    pub fn serialize_element(&mut self, ele: &EleAddrErased) -> Option<serde_json::Value> {
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
    pub fn deserialize_scene(&mut self, man: &mut Manager, content: serde_json::Value) -> Result<SceneDeserResult, SceneSerdeError> {
        #[derive(Deserialize, Clone)]
        struct EleObj {
            name: String,
            payload: serde_json::Value
        }

        #[derive(Deserialize, Clone)]
        struct EntObj {
            name: String,
            parent_payload: serde_json::Value, // This is a serialized form of EntAddr
            id: i64,
            eles: Vec<EleObj>
        }

        struct EntDeserializeState {
            payload: EntObj,
            addr: EntAddr
        }
        
        begin_deserialize();

        // Deserialize all entity data, create the actual entities, and associate the original data with the entities
        let ent_states: Vec<EntDeserializeState> =
        serde_json::from_value::<Vec<EntObj>>(content)
        .map_err(|er| SceneSerdeError::SerdeError(er))?
        .into_iter()
        .map(|payload| {
            let addr = set_mapping(Uuid::from_u128(payload.id as u128), payload.name.clone(), man);
            EntDeserializeState { payload, addr }
        })
        .collect();
        
        let mut reparent_failures = Vec::<String>::new();
        // All entities have been created; we can now assign parent/child relations
        ent_states.iter().for_each(|state| {
            let parent_addr = serde_json::from_value::<EntAddr>(state.payload.parent_payload.clone()).unwrap();
            let child_addr = map_id(Uuid::from_u128(state.payload.id as u128));
            if let Err(_er) = man.reparent(child_addr.clone(), parent_addr.clone()) {
                let child_ent = child_addr.get_ref().unwrap();
                let parent_ent = parent_addr.get_ref().unwrap();
                reparent_failures.push(format!("Making Child -> Parent relationship \"{}\" -> \"{}\" would have created a cycle", child_ent.name, parent_ent.name));
            }
        });

        if reparent_failures.len() > 0 {
            // Clear out created entities
            ent_states.into_iter().for_each(|state| {
                man.destroy_entity(state.addr);
            });
            man.resolve();
            end_deserialize();
            return Err(SceneSerdeError::CycleError(reparent_failures.join("\n")));
        }

        // First create empty elements in their respective entities so no EleAddr deserialize
        // fails due to the element not yet being added
        struct EleAddrDeserializeState {
            ele: EleAddrErased,
            payload: serde_json::Value
        }

        let deser_attempts =
        ent_states.iter().map(|pair| {
            pair.payload.eles.iter().map(|ele_obj| {
                self
                .deserialize_empty_into(pair.addr.clone(), ele_obj.name.clone())
                .map(|ele| EleAddrDeserializeState {
                    ele,
                    payload: ele_obj.payload.clone()
                })
            })
            .collect::<Vec<Result<EleAddrDeserializeState, SceneSerdeError>>>()
        })
        .flatten()
        .collect::<Vec<Result<EleAddrDeserializeState, SceneSerdeError>>>();

        let ecs_deser_errors: Vec<SceneSerdeError> =
        deser_attempts
        .iter()
        .filter(|attempt| attempt.is_ok())
        .map(|attempt| attempt.as_ref().ok().unwrap())
        .map(|state| {
            state.ele.clone().get_ref_mut().unwrap().ecs_deserialize(state.payload.clone())
        })
        .filter(|state_attempt| state_attempt.is_err())
        .map(|state_attempt| SceneSerdeError::SerdeError(state_attempt.err().unwrap()))
        .collect();

        end_deserialize();

        let errors =
        deser_attempts
        .into_iter()
        .filter(|state| state.is_err())
        .map(|state| state.err().unwrap())
        .chain(ecs_deser_errors.into_iter())
        .collect();

        Ok(SceneDeserResult {
            ents: ent_states.into_iter().map(|pair| pair.addr).collect(),
            errors
        })
    }
    pub fn serialize_scene(&mut self, _man: &mut Manager, content: Vec<EntAddr>) -> serde_json::Value {
        #[derive(Serialize)]
        struct EntObj {
            name: String,
            parent_payload: serde_json::Value,
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
                        self.serialize_element(&ele)
                    })
                    .collect();

                EntObj {
                    name: ea.get_ref().unwrap().name.clone(),
                    parent_payload: serde_json::to_value(ea.get_ref().unwrap().get_parent()).unwrap(),
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