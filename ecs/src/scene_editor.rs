use std::{any::TypeId, cell::RefCell, fs, rc::Rc};
use uuid::Uuid;
use imgui::*;
use std::collections::HashSet;

use crate::editor_helpers;
use crate::entity::*;
use crate::scene_serde::*;

fn uuid_truncated(id: Uuid) -> String {
    id.to_string().chars().take(8).collect::<String>()
}

#[derive(Clone)]
struct SelectedEnt {
    addr: EntAddr,
    selected_element: Option<TypeId>,
    selected_element_label: String,
    creator_search: String
}

impl SelectedEnt {
    pub fn new(addr: EntAddr) -> Self {
        Self {
            addr,
            selected_element: None,
            selected_element_label: String::new(),
            creator_search: "".to_string()
        }
    }
}

pub struct SceneEditor {
    selected_list: Vec<Rc<RefCell<SelectedEnt>>>,
    ents_expanded: HashSet<EntAddr>,
}

impl SceneEditor {
    pub fn new() -> Self {
        Self {
            selected_list: Vec::new(),
            ents_expanded: HashSet::new()
        }
    }
    fn save_scene(&mut self, scene: &mut SceneSerde, man: &mut Manager, name: &str) -> Result<(), std::io::Error> {
        let all_ents = man.all_entities();
        let val = scene.serialize_scene(man, all_ents);
        fs::write(name, serde_json::to_string(&val).unwrap())
    }
    fn load_scene(&mut self, scene: &mut SceneSerde, man: &mut Manager, name: &str) {
        let st = fs::read_to_string(name).unwrap();
        let val = serde_json::from_str(&st).unwrap();
        scene.deserialize_scene(man, val).unwrap();
    }
    
    fn render_ui_for_ent(&mut self, ui: &Ui, scene: &mut SceneSerde, man: &mut Manager, selected: Rc<RefCell<SelectedEnt>>) -> bool {
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

            {
                let mut parent = ent_addr.get_ref().unwrap().get_parent();
                if editor_helpers::select_entity(&mut parent, "Parent", ui, man) {
                    if let Err(_err) = man.reparent(ent_addr.clone(), parent.clone()) {
                        println!("Reparenting would have created a cycle")
                    };
                }
            }

            ui.separator();
            ui.input_text(":Search Elements", &mut (*selected).borrow_mut().creator_search).build();
            let list = scene.find_creators((*selected).borrow().creator_search.as_str());
            for entry in list.iter() {
                let mut select_pos: Option<[f32; 2]> = None;
                let style = ui.push_style_color(StyleColor::ButtonActive, [1_f32, 1_f32, 1_f32, 1_f32]);
                if ent_addr.get_ref_mut().unwrap().query_element_addr_by_id(&entry.id).valid() {
                    let (style0, style1) = (
                        ui.push_style_color(StyleColor::Button, [0.5_f32, 0_f32, 0_f32, 1_f32]),
                        ui.push_style_color(StyleColor::ButtonHovered, [1_f32, 0.5_f32, 0.5_f32, 1_f32])
                    );
                    select_pos = Some(ui.cursor_pos());
                    if ui.button_with_size(&*ImString::new(("Destroy ".to_owned() + entry.name.as_str()).as_str()), [200_f32, 20_f32]) {
                        let ele_addr = ent_addr.get_ref_mut().unwrap().query_element_addr_by_id(&entry.id);
                        man.destroy_element(ele_addr);

                        man.resolve();

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

                let mut ele_addr = ent_addr.clone().get_ref_mut().unwrap().query_element_addr_by_id(&selected_id);
                if let Some(mut ele) = ele_addr.get_ref_mut() {
                    ele.fill_ui(ui, man);
                }
            }
        });

        opened
    }

    fn render_ent_recurse(&mut self, ui: &Ui, man: &mut Manager, ent: EntAddr, level: i32) {
        let cursor = ui.cursor_pos();
        let id_token = ui.push_id(ent.get_ref().unwrap().get_id().to_string());

        // Collapse or expand the entity hierarchy
        let needs_expansion_button = ent.get_ref().unwrap().get_children().len() > 0;
        let change_expansion = needs_expansion_button && ui.button_with_size("", [20_f32, 20_f32]);
        let show_children = self.ents_expanded.contains(&ent);

        if change_expansion {
            if show_children {
                self.ents_expanded.remove(&ent);
            } else {
                self.ents_expanded.insert(ent.clone());
            }
        }

        ui.set_cursor_pos([cursor[0] + 30f32 + (level * 30) as f32, cursor[1]]);
        if ui.button_with_size(format!("Select \"{}\"", ent.get_ref().unwrap().name), [250_f32, 20_f32]) {
            if !self.selected_list.iter().any(|e| (**e).borrow().addr == ent) {
                self.selected_list.push(Rc::new(RefCell::new(SelectedEnt::new(ent.clone()))));
            }
        }
        id_token.pop();

        ui.set_cursor_pos([cursor[0] + 300_f32 + (level * 30) as f32, cursor[1]]);
        if ui.button_with_size(format!("Destroy {}", uuid_truncated(ent.get_ref().unwrap().get_id())), [130_f32, 20_f32]) {
            man.destroy_entity(ent.clone());
        }

        let children = ent.get_ref().unwrap().get_children();
        if show_children {
            for child in children {
                self.render_ent_recurse(ui, man, child, level + 1);
            }
        }
    }

    pub fn render(&mut self, ui: &Ui, scene: &mut SceneSerde, man: &mut Manager) {
        let mut new_selected = Vec::new();
        for ent in self.selected_list.iter() {
            if (**ent).borrow().addr.valid() {
                new_selected.push(ent.clone());
            }
        }
        self.selected_list = new_selected;

        Window::new(ui,"Manager")
        .position([0.0, 0.0], Condition::Always)
        .build(|| {
            if ui.button_with_size("Load Scene", [200_f32, 20_f32]) {
                self.load_scene(scene, man, "./test.json");
                /*
                if let Ok(to_load) = nfd::open_file_dialog(Some("json"), None) {
                    match to_load {
                        nfd::Response::Okay(file) => {
                            self.load_scene(scene, man, file.as_str())
                        },
                        _ => println!("eh")
                    };
                }*/
            }

            if ui.button_with_size("Save Scene", [200_f32, 20_f32]) {
                self.save_scene(scene, man, "./test.json").unwrap();
                /*
                if let Ok(to_load) = nfd::open_save_dialog(Some("json"), None) {
                    match to_load {
                        nfd::Response::Okay(file) => {
                            self.save_scene(scene, man, file.as_str()).unwrap()
                        },
                        _ => println!("eh")
                    };
                }*/
            }
            if ui.button_with_size("Create Entity", [250_f32, 20_f32]) {
                man.create_entity(String::new());
            }
            
            for ent in man.root_entities().iter() {
                self.render_ent_recurse(ui, man, ent.clone(), 0);
            }
        });
        
        let mut removed = Vec::new();
        for i in 0..self.selected_list.len() {
            if !self.render_ui_for_ent(ui, scene,  man, self.selected_list[i].clone()) {
                removed.push(self.selected_list[i].clone());
            }
        }

        for to_remove in removed.into_iter() {
            self.selected_list.remove(self.selected_list.iter().position(|selected| (**selected).borrow().addr == (*to_remove).borrow().addr).unwrap());
        }
    }
}