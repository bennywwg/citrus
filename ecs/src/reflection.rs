use imgui::*;
use crate::{element::{EleAddr, Element}, entity::*};

fn find_entities(man: &mut Manager, name: &str) -> Vec<EntAddr> {
    let mut res = Vec::new();
    for b in man.all_entities().iter() {
        let ent_name = b.get_ref().unwrap().name.clone();
        if ent_name.contains(name) {
            res.push(b.clone());
        }
    }
    res
}

pub fn select_entity(res: &mut EntAddr, label: &str, ui: &Ui, man: &mut Manager) -> bool {
    //let mut search_str = String::new();
    //ui.input_text("Search Entities", &mut search_str).build();

    let mut entities = find_entities(man, "");//&search_str);
    entities.insert(0, EntAddr::new());
    let ents_names: Vec<String>
    =entities.iter()
    .map(|ent| {
        if ent.valid() {
            let r = ent.get_ref().unwrap();
        
            format!("\"{}\",{}",
                r.name,
                r.get_id().to_string().chars().take(8).collect::<String>()
            )
        } else {
            "(Null)".to_string()
        }
    })
    .collect();

    
    let mut val: usize
    =entities.iter()
    .position(|e| {
        if !e.valid() && !res.valid() {
            true
        } else if e.valid() && res.valid() {
            e.get_ref().unwrap().get_id() == res.get_ref().unwrap().get_id()
        } else {
            false
        }
    }).unwrap_or(0);
    let selected = ui.combo_simple_string(label, &mut val, ents_names.as_slice());
    if selected {
        *res = entities[val].clone();
    }
    selected
}

pub fn select_element<T: Element>(res: &mut EleAddr<T>, label: &str, ui: &Ui, man: &mut Manager) -> bool {
    //let mut search_str = String::new();
    //ui.input_text("Search Entities", &mut search_str).build();

    let mut entities = find_entities(man, "");//&search_str);
    entities.insert(0, EntAddr::new());
    let ents_names: Vec<String>
    =entities.iter()
    .map(|ent| {
        if ent.valid() {
            let mut r = ent.get_ref_mut().unwrap();

            let warn = match r.query_element::<T>() {
                Some(_) => "",
                None => "[!]"
            };
        
            format!("{}\"{}\",{}",
                warn,
                r.name,
                r.get_id().to_string().chars().take(8).collect::<String>()
            )
        } else {
            "(Null)".to_string()
        }
    })
    .collect();

    
    let mut val: usize
    =entities.iter()
    .position(|e| {
        if !e.valid() && !res.valid() {
            true
        } else if e.valid() && res.valid() {
            e.get_ref().unwrap().get_id() == res.get_owner().get_ref().unwrap().get_id()
        } else {
            false
        }
    }).unwrap_or(0);
    let selected = ui.combo_simple_string(label, &mut val, ents_names.as_slice());
    if selected {
        *res = entities[val].get_ref_mut().unwrap().query_element_addr::<T>();
    }
    selected
}