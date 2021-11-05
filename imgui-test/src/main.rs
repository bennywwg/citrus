#[path="../support/mod.rs"]
mod support;

use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use __core::cell::RefCell;
use ecs::deserialize_context::*;
use imgui::*;
use ecs::{element::*, entity::*};
use uuid::Uuid;
use serde::*;

fn uuid_truncated(id: Uuid) -> String {
    id.to_string().chars().take(8).collect::<String>()
}

fn im_id<'a>(uuid: Uuid, ui: &Ui) -> imgui::Id {
    Id::Int(uuid.as_u128() as i32, ui)
}

#[derive(Clone)]
struct CreatorEntry {
    creator: Rc<Box<dyn Fn(EntAddr) -> EleAddrErased>>,
    name: String,
    id: TypeId
}

#[derive(Clone)]
struct SelectedEnt {
    addr: EntAddr,
    selected_element: Option<TypeId>,
    selected_element_label: String
}

impl SelectedEnt {
    pub fn new(addr: EntAddr) -> Self {
        Self {
            addr,
            selected_element: None,
            selected_element_label: String::new()
        }
    }
}

struct ManagerEditor {
    entity_search: String,
    creator_map: HashMap<TypeId, CreatorEntry>,
    creator_search: String,
    selected_list: Vec<Rc<RefCell<SelectedEnt>>>
}

impl ManagerEditor {
    pub fn new() -> Self {
        Self {
            entity_search: "".to_string(),
            creator_map: HashMap::new(),
            creator_search: "".to_string(),
            selected_list: Vec::new()
        }
    }
    pub fn register_element_creator<T>(&mut self, default: T, name: &str) where
        T: Element,
        T: Clone,
        T: Any,
        T: Serialize,
        T: Deserialize<'static>
    {
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

    fn deserialize_element_into(&mut self, ent: EntAddr, val: serde_json::Value) -> EleAddrErased {
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
    fn serialize_element(&mut self, ele: EleAddrErased) -> Option<serde_json::Value> {
        #[derive(Serialize)]
        struct ElementObj {
            name: String,
            payload: serde_json::Value
        }

        if !ele.valid() {
            return None;
        }

        // TODO: Use this
        //let tid = ele.get_element_type_id().unwrap();

        let creator = self.find_exact_creator_by_id(ele.get_element_type_id().unwrap()).unwrap();

        Some(serde_json::to_value(ElementObj {
            name: creator.name,
            payload: ele.get_ref().unwrap().ecs_serialize()
        }).unwrap())
    }
    fn deserialize_scene(&mut self, man: &mut Manager, content: serde_json::Value) -> Option<Vec<EntAddr>> {
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
    fn serialize_scene(&mut self, _man: &mut Manager, content: Vec<EntAddr>) -> serde_json::Value {
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
    
    
    fn find_creators(&self, name: &str) -> Vec<CreatorEntry> {
        let mut res = Vec::<CreatorEntry>::new();
        for b in self.creator_map.iter() {
            if b.1.name.contains(name) {
                res.push(b.1.clone());
            }
        }
        res
    }
    fn find_exact_creator(&self, name: &str) -> Option<CreatorEntry> {
        for b in self.creator_map.iter() {
            if b.1.name == name {
                return Some(b.1.clone());
            }
        }
        None
    }
    fn find_exact_creator_by_id(&self, id: TypeId) -> Option<CreatorEntry> {
        for b in self.creator_map.iter() {
            if b.1.id == id {
                return Some(b.1.clone());
            }
        }
        None
    }
    fn find_entities(&self, man: &mut Manager, name: &str) -> Vec<EntAddr> {
        let mut res = Vec::new();
        for b in man.all_entities().iter() {
            let ent_name = b.get_ref_mut().unwrap().name.clone();
            if ent_name.contains(name) {
                res.push(b.clone());
            }
        }
        res
    }
    fn save_scene(&mut self, man: &mut Manager, name: &str) -> Result<(), std::io::Error> {
        let all_ents = man.all_entities();
        let val = self.serialize_scene(man, all_ents);
        fs::write(name, serde_json::to_string(&val).unwrap())
    }
    fn load_scene(&mut self, man: &mut Manager, name: &str) {
        let st = fs::read_to_string(name).unwrap();
        let val = serde_json::from_str(&st).unwrap();
        println!("{:?}", self.deserialize_scene(man, val).unwrap().len());
    }
    fn render_ui_for_ent(&mut self, ui: &Ui, man: &mut Manager, selected: Rc<RefCell<SelectedEnt>>) -> bool {
        let ent_addr = (*selected).borrow().addr.clone();
        
        if !ent_addr.valid() {
            return false;
        }

        let truncated_id = format!("{}", ent_addr.get_ref_mut().unwrap().get_id().to_string());

        let mut opened: bool = true;
        Window::new(ui, &*ImString::new(truncated_id.as_str()))
        .collapsible(true)
        .resizable(true)
        .size([400.0, 400.0], Condition::FirstUseEver)
        .opened(&mut opened)
        .build(move || {
            {
                ui.input_text(":Name", &mut ent_addr.get_ref_mut().unwrap().name).build();
            }

            ui.separator();
            ui.input_text(":Search Elements", &mut self.creator_search).build();
            let list = self.find_creators(self.creator_search.as_str());
            for entry in list.iter() {
                let mut select_pos: Option<[f32; 2]> = None;
                let style = ui.push_style_color(StyleColor::ButtonActive, [1_f32, 1_f32, 1_f32, 1_f32]);
                if ent_addr.get_ref_mut().unwrap().query_element_addr_by_id(entry.id).valid() {
                    let (style0, style1) = (
                        ui.push_style_color(StyleColor::Button, [0.5_f32, 0_f32, 0_f32, 1_f32]),
                        ui.push_style_color(StyleColor::ButtonHovered, [1_f32, 0.5_f32, 0.5_f32, 1_f32])
                    );
                    select_pos = Some(ui.cursor_pos());
                    if ui.button_with_size(&*ImString::new(("Destroy ".to_owned() + entry.name.as_str()).as_str()), [200_f32, 20_f32]) {
                        ent_addr.get_ref_mut().unwrap().remove_element_by_id(entry.id).expect("Should have removed element");

                        if  (*selected).borrow().selected_element == Some(entry.id) {
                            (*selected).borrow_mut().selected_element = None;
                            (*selected).borrow_mut().selected_element_label = "(None)".to_string();
                        }
                    }
                    style1.pop();
                    style0.pop();
                } else {
                    if ui.button_with_size(&*ImString::new(("Create  ".to_owned() + entry.name.as_str()).as_str()), [200_f32, 20_f32]) {
                        assert!((*entry.creator)(ent_addr.clone()).valid());
                    }
                }
                style.pop();
                
                if let Some(cursor) = select_pos {
                    ui.set_cursor_pos([cursor[0] + 220_f32, cursor[1]]);
                    let style = match (*selected).borrow().selected_element == Some(entry.id) {
                        true => Some(ui.push_style_color(StyleColor::Button, [0_f32, 0.5_f32, 0_f32, 1_f32])),
                        false => None
                    };
                    if ui.button_with_size(format!("Select {}", entry.name), [150_f32, 20_f32]) {
                        (*selected).borrow_mut().selected_element = Some(entry.id);
                        (*selected).borrow_mut().selected_element_label = entry.name.clone();
                    }
                    if let Some(st) = style {
                        st.pop();
                    }
                }
            }

            if let Some(selected_id) = (*selected).borrow().selected_element {
                ui.text(format!("Selected {}", (*selected).borrow().selected_element_label));
                ui.separator();

                let mut ele_addr = ent_addr.clone().get_ref_mut().unwrap().query_element_addr_by_id(selected_id);
                if let Some(mut ele) = ele_addr.get_ref_mut() {
                    ele.fill_ui(ui, man);
                }
            }
        });

        opened
    }
    pub fn render(&mut self, ui: &Ui, man: &mut Manager) {
        let mut new_selected = Vec::new();
        for ent in self.selected_list.iter() {
            if (**ent).borrow().addr.valid() {
                new_selected.push(ent.clone());
            }
        }
        self.selected_list = new_selected;

        Window::new(ui,"Manager")
        .size([400.0, 400.0], Condition::FirstUseEver)
        .build(|| {
            if ui.button_with_size("Load Scene", [200_f32, 20_f32]) {
                if let Ok(to_load) = nfd::open_file_dialog(Some("json"), None) {
                    match to_load {
                        nfd::Response::Okay(file) => {
                            self.load_scene(man, file.as_str())
                        },
                        _ => println!("eh")
                    };
                }
            }

            if ui.button_with_size("Save Scene", [200_f32, 20_f32]) {
                if let Ok(to_load) = nfd::open_save_dialog(Some("json"), None) {
                    match to_load {
                        nfd::Response::Okay(file) => {
                            self.save_scene(man, file.as_str()).unwrap()
                        },
                        _ => println!("eh")
                    };
                }
            }
            if ui.button_with_size("Create Entity", [250_f32, 20_f32]) {
                man.create_entity(String::new());
            }
            ui.separator();
            ui.input_text(":Search Entities", &mut self.entity_search).build();
            ui.separator();

            for ent in self.find_entities(man, self.entity_search.as_str()).iter() {
                let cursor = ui.cursor_pos();
                let id_token = ui.push_id(ent.get_ref().unwrap().get_id().to_string());
                if ui.button_with_size(format!("Select \"{}\"", ent.get_ref().unwrap().name), [250_f32, 20_f32]) {
                    if !self.selected_list.iter().any(|e| (**e).borrow().addr == *ent) {
                        self.selected_list.push(Rc::new(RefCell::new(SelectedEnt::new(ent.clone()))));
                    }
                }
                id_token.pop();

                ui.set_cursor_pos([cursor[0] + 270_f32, cursor[1]]);                
                if ui.button_with_size(format!("Destroy {}", uuid_truncated(ent.get_ref().unwrap().get_id())), [130_f32, 20_f32]) {
                    man.destroy_entity(ent.clone());
                }
            }
        });
        
        let mut removed = Vec::new();
        for i in 0..self.selected_list.len() {
            if !self.render_ui_for_ent(ui, man, self.selected_list[i].clone()) {
                removed.push(self.selected_list[i].clone());
            }
        }

        for to_remove in removed.into_iter() {
            self.selected_list.remove(self.selected_list.iter().position(|selected| (**selected).borrow().addr == (*to_remove).borrow().addr).unwrap());
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
    fn fill_ui(&mut self, ui: &imgui::Ui, _man: &mut Manager) {
        ui.text("Ayyyy!");
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct B {
    val: i32,
    other: EntAddr
}

impl Element for B {
    fn update(&mut self, _man: &mut Manager, _owner: EntAddr) {
        println!("B: val = {}", self.val);
        self.val += 10;
    }
    fn ecs_serialize(&self) -> serde_json::Value {
        serde_json::to_value(&self).unwrap()
    }
    fn ecs_deserialize(&mut self, data: serde_json::Value) {
        match serde_json::from_value::<Self>(data) {
            Ok(parsed) => *self = parsed,
            Err(er) => println!("{:?}", er)
        };
    }
    
    fn fill_ui(&mut self, ui: &imgui::Ui, man: &mut Manager) {
        ecs::reflection::select_entity(&mut self.other, ui, man);
    }
}

fn main() {
    /*
    println!("{:?}", match nfd::open_file_dialog(None, None).unwrap() {
        nfd::Response::Okay(val) => val,
        _ => panic!()
    });
    */

    //let a: EntAddr = serde_json::from_value(serde_json::Value::Null).unwrap();

    let mut manager = Manager::new();

    let _ent = manager.create_entity("Baba booie".to_string());

    //manager.update();

    let mut ed = ManagerEditor::new();

    ed.register_element_creator(A { val: 0 }, "PosRot");
    ed.register_element_creator(B { val: 2, other: EntAddr::new() }, "Element B");

    let system = support::init(file!());
    system.main_loop(move |_, ui| {
        ed.render(ui, &mut manager);
        manager.resolve();
    });
}