use uuid::Uuid;
use std::{collections::HashMap, sync::RwLock};

use crate::entity::{EntAddr, Manager};

// I'm too *lazy* to make a stateful deserializer so this static state must suffice
// no unsafe tho
lazy_static! {
    static ref ID_MAP: RwLock<HashMap<Uuid, EntAddr>> = RwLock::new(HashMap::new());
    static ref IN_DESERIALIZE: RwLock<bool> = RwLock::new(false);
}

pub fn begin_deserialize() {
    assert!(!*IN_DESERIALIZE.read().unwrap());
    *IN_DESERIALIZE.write().unwrap() = true;
}

pub fn end_deserialize() {
    assert!(*IN_DESERIALIZE.read().unwrap());
    *IN_DESERIALIZE.write().unwrap() = false;
    *ID_MAP.write().unwrap() = HashMap::new();
}

pub fn map_id(id: Uuid) -> EntAddr {
    assert!(*IN_DESERIALIZE.read().unwrap());
    
    match id.as_u128() == 0 {
        true => EntAddr::new(),
        false => {
            match (*ID_MAP.write().unwrap()).entry(id) {
                std::collections::hash_map::Entry::Occupied(existing) => existing.get().clone(),
                std::collections::hash_map::Entry::Vacant(_) => EntAddr::new()
            }
        }
    }
}

pub fn set_mapping(id_ser: Uuid, name: String, man: &mut Manager) -> EntAddr {
    assert!(*IN_DESERIALIZE.read().unwrap());
    assert!(id_ser.as_u128() != 0);

    assert!(!(*ID_MAP.write().unwrap()).contains_key(&id_ser));

    let res = man.create_entity(name);

    // TODO: assert that it doesn't contain the value either
    (*ID_MAP.write().unwrap()).insert(id_ser, res.clone());

    res
}