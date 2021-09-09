#[path="../support/mod.rs"]
mod support;

use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::rc::Rc;
use __core::borrow::Borrow;
use __core::cell::RefCell;
use imgui::*;
use ecs::{element::*, entity::*};
use uuid::Uuid;
use serde::*;

fn uuid_truncated(id: Uuid) -> String {
    id.to_string().chars().take(8).collect::<String>()
}

fn im_id<'a>(uuid: Uuid) -> imgui::Id<'a> {
    Id::Int(uuid.as_u128() as i32)
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
    fn find_creators(&self, name: &str) -> Vec<CreatorEntry> {
        let mut res = Vec::<CreatorEntry>::new();
        for b in self.creator_map.iter() {
            if b.1.name.contains(name) {
                res.push(b.1.clone());
            }
        }
        res
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
    fn render_ui_for_ent(&mut self, ui: &Ui, selected: Rc<RefCell<SelectedEnt>>) -> bool {
        let ent_addr = (*selected).borrow().addr.clone();
        
        if !ent_addr.valid() {
            return false;
        }

        let truncated_id = format!("{}", ent_addr.get_ref_mut().unwrap().get_id().to_string());

        let mut opened: bool = true;
        Window::new(&*ImString::new(truncated_id.as_str()))
        .collapsible(true)
        .resizable(true)
        .size([400.0, 400.0], Condition::FirstUseEver)
        .opened(&mut opened)
        .build(ui, move || {
            {
                let mut ent_name_buf = ImString::new(ent_addr.get_ref().unwrap().name.as_str());
                if ui.input_text(im_str!(":Name"), &mut ent_name_buf)
                .resize_buffer(true)
                .build() {
                    ent_addr.get_ref_mut().unwrap().name = ent_name_buf.to_string();
                }
            }

            ui.separator();
            let mut buf = ImString::new(self.creator_search.as_str());
            if ui.input_text(im_str!(":Search Elements"), &mut buf)
            .resize_buffer(true)
            .build() {
                self.creator_search = buf.to_string();
            }
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
                    if ui.button(&*ImString::new(("Destroy ".to_owned() + entry.name.as_str()).as_str()), [200_f32, 20_f32]) {
                        ent_addr.get_ref_mut().unwrap().remove_element_by_id(entry.id).expect("Should have removed element");

                        if  (*selected).borrow().selected_element == Some(entry.id) {
                            (*selected).borrow_mut().selected_element = None;
                            (*selected).borrow_mut().selected_element_label = "(None)".to_string();
                        }
                    }
                    style1.pop(ui);
                    style0.pop(ui);
                } else {
                    if ui.button(&*ImString::new(("Create  ".to_owned() + entry.name.as_str()).as_str()), [200_f32, 20_f32]) {
                        assert!((*entry.creator)(ent_addr.clone()).valid());
                    }
                }
                style.pop(ui);
                
                if let Some(cursor) = select_pos {
                    ui.set_cursor_pos([cursor[0] + 220_f32, cursor[1]]);
                    let style = match (*selected).borrow().selected_element == Some(entry.id) {
                        true => Some(ui.push_style_color(StyleColor::Button, [0_f32, 0.5_f32, 0_f32, 1_f32])),
                        false => None
                    };
                    if ui.button(&*im_str!("Select {}", entry.name), [150_f32, 20_f32]) {
                        (*selected).borrow_mut().selected_element = Some(entry.id);
                        (*selected).borrow_mut().selected_element_label = entry.name.clone();
                    }
                    if let Some(st) = style {
                        st.pop(ui);
                    }
                }
            }

            if let Some(selected_id) = (*selected).borrow().selected_element {
                ui.text(im_str!("Selected {}", (*selected).borrow().selected_element_label));
                ui.separator();
                if let Some(mut ele) = ent_addr.clone().get_ref_mut().unwrap().query_element_addr_by_id(selected_id).get_ref_mut() {
                    ele.fill_ui(ui);
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

        Window::new(im_str!("Manager"))
        .size([400.0, 400.0], Condition::FirstUseEver)
        .build(ui, || {
            if ui.button(im_str!("Create Entity"), [250_f32, 20_f32]) {
                man.create_entity(String::new());
            }
            ui.separator();
            let mut buf = ImString::new(self.entity_search.as_str());
            if ui.input_text(im_str!(":Search Entities"), &mut buf)
            .resize_buffer(true)
            .build() {
                self.entity_search = buf.to_string();
            }
            ui.separator();

            for ent in self.find_entities(man, self.entity_search.as_str()).iter() {
                let cursor = ui.cursor_pos();
                let id_token = ui.push_id(im_id(ent.get_ref().unwrap().get_id()));
                if ui.button(&*im_str!("Select \"{}\"", ent.get_ref().unwrap().name), [250_f32, 20_f32]) {
                    if !self.selected_list.iter().any(|e| (**e).borrow().addr == *ent) {
                        self.selected_list.push(Rc::new(RefCell::new(SelectedEnt::new(ent.clone()))));
                    }
                }
                id_token.pop(ui);

                ui.set_cursor_pos([cursor[0] + 270_f32, cursor[1]]);                
                if ui.button(&*im_str!("Destroy {}", uuid_truncated(ent.get_ref().unwrap().get_id())), [130_f32, 20_f32]) {
                    man.destroy_entity(ent.clone());
                }
            }
        });
        
        let mut removed = Vec::new();
        for i in 0..self.selected_list.len() {
            if !self.render_ui_for_ent(ui, self.selected_list[i].clone()) {
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
    fn fill_ui(&mut self, ui: &imgui::Ui) {
        ui.text(im_str!("Ayyyy!"));
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct B {
    val: i32
}

impl Element for B {
    fn update(&mut self, _man: &mut Manager, _owner: EntAddr) {
        println!("B: val = {}", self.val);
        self.val += 10;
    }
}

fn main() {
    let mut manager = Manager::new();

    let ent = manager.create_entity("Baba booie".to_string());

    //manager.update();

    let mut ed = ManagerEditor::new();

    ed.register_element_creator(A { val: 0 }, "PosRot");
    ed.register_element_creator(B { val: 2 }, "Element B");

    let system = support::init(file!());
    system.main_loop(move |_, ui| {
        ed.render(ui, &mut manager);
        manager.resolve();
    });
}