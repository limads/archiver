/*Copyright (c) 2022 Diego da Silva Lima. All rights reserved.

This work is licensed under the terms of the MIT license.  
For a copy, see <https://opensource.org/licenses/MIT>.*/

use serde::{Serialize, Deserialize, de::DeserializeOwned};
use gtk4::*;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use std::fs::File;

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
    state.width = win.allocation().width();
    state.height = win.allocation().height();
}

pub fn load_shared_serializable<T : DeserializeOwned>(path : &str) -> Option<Rc<RefCell<T>>> {
    match File::open(path) {
        Ok(f) => {
            let state : Result<T, _> = serde_json::from_reader(f);
            match state {
                Ok(s) => {
                    Some(Rc::new(RefCell::new(s)))
                },
                Err(e) => {
                    eprintln!("Could not load configuration: {}", e);
                    None
                }
            }
        },
        Err(e) => {
            eprintln!("Could not load configuration: {}", e);
            None
        }
    }
}

pub fn save_shared_serializable<T : Serialize + Send + Clone + 'static>(
    state : &Rc<RefCell<T>>,
    path : &str
) -> thread::JoinHandle<bool> {
    let state = state.borrow().clone();
    let path = path.to_string();
    thread::spawn(move|| {
        match File::create(&path) {
            Ok(f) => {
                match serde_json::to_writer_pretty(f, &state) {
                    Ok(_) => true,
                    Err(e) => {
                        eprintln!("Could not save configuration: {}", e);
                        false
                    }
                }
            },
            Err(e) => {
                eprintln!("Could not save configuration: {}", e);
                false
            }
        }
    })
}

