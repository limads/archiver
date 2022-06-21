use serde::{Serialize, Deserialize, de::DeserializeOwned};
use gtk4::*;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use std::fs::File;
use std::io::Read;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WindowState {
    pub width : i32,
    pub height : i32
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PanedState {
    pub primary : i32,
    pub secondary : i32
}

pub fn set_paned_on_close(primary : &Paned, secondary : &Paned, state : &mut PanedState) {
    state.primary = primary.position();
    state.secondary = secondary.position();
}

pub fn set_win_dims_on_close(win : &ApplicationWindow, state : &mut WindowState) {
    state.width = win.allocation().width;
    state.height = win.allocation().height;
}


pub fn load_shared_serializable<T : DeserializeOwned>(path : &str) -> Option<Rc<RefCell<T>>> {
    let state : T = serde_json::from_reader(File::open(path).ok()?).ok()?;
    Some(Rc::new(RefCell::new(state)))
}

pub fn save_shared_serializable<T : Serialize + Send + Clone + 'static>(
    state : &Rc<RefCell<T>>,
    path : &str
) -> thread::JoinHandle<bool> {
    let state = state.borrow().clone();
    let path = path.to_string();

    // TODO filter repeated scripts and connections

    thread::spawn(move|| {
        if let Ok(f) = File::create(&path) {
            serde_json::to_writer_pretty(f, &state).is_ok()
        } else {
            false
        }
    })
}

