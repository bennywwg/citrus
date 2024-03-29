#[path="../support/mod.rs"]
mod support;

use citrus_ecs::{element::*, entity::*, scene_editor::*, scene_serde::*};
use serde::*;
use imgui_glium_renderer::imgui as imgui;

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
    other: EntAddr,
    ele: EleAddr<A>
}

impl Element for B {
    fn update(&mut self, _man: &mut Manager, _owner: EntAddr) {
        println!("B: val = {}", self.val);
        self.val += 10;
    }
    fn fill_ui(&mut self, ui: &imgui::Ui, man: &mut Manager) {
        citrus_ecs::editor_helpers::select_entity(&mut self.other, "other", ui, man);
        citrus_ecs::editor_helpers::select_element(&mut self.ele, "ele", ui, man);
    }
}

fn main() {
    let mut ma =        Manager::new();
    let mut se =        SceneSerde::new();
    let mut ed =        SceneEditor::new();

    let a = ma.create_entity("Baba booie".to_string());
    let b = ma.create_entity("Child".to_string());
    ma.reparent(a, b).unwrap();

    se.register_element_creator(A { val: 0 }, "PosRot");
    se.register_element_creator(B { val: 2, other: EntAddr::new(), ele: EleAddr::new() }, "Element B");

    let system = support::init(file!());
    system.main_loop(move |_, ui| {
        ed.render(ui, &mut se, &mut ma);
        ma.resolve();
    });
}