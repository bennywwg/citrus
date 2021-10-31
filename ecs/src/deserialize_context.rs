use uuid::Uuid;
use std::{collections::HashMap, sync::RwLock};

// I'm too *lazy* to make a stateful deserializer so this static state must suffice
// no unsafe tho
lazy_static! {
    static ref ID_MAP: RwLock<HashMap<Uuid, Uuid>> = RwLock::new(HashMap::new());
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

pub fn map_id(id: Uuid) -> Uuid {
    assert!(*IN_DESERIALIZE.read().unwrap());
    
    match id.as_u128() == 0 {
        true => Uuid::from_u128(0),
        false => {
            (*ID_MAP.write().unwrap())
            .entry(id)
            .or_insert(Uuid::new_v4())
            .clone()
        }
    }
}

pub fn set_mapping(id_ser: Uuid, id_scene: Uuid) {
    assert!(*IN_DESERIALIZE.read().unwrap());
    assert!(id_ser.as_u128() != 0);
    assert!(id_scene.as_u128() != 0);

    assert!(!(*ID_MAP.write().unwrap()).contains_key(&id_ser));
    // TODO: assert that it doesn't contain the value either
    (*ID_MAP.write().unwrap()).insert(id_ser, id_scene);
}