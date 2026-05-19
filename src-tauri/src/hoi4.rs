//! HOI4 content models (Focus Trees, Events, Scripted GUIs, Decisions, etc.)

pub mod focus_tree;
pub mod scripted_gui;
pub mod event;
pub mod decision;

// Base traits or common models for HOI4 mod components
pub trait Hoi4Component {
    fn to_script(&self) -> String;
}
