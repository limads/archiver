/*Copyright (c) 2022 Diego da Silva Lima. All rights reserved.

This work is licensed under the terms of the MIT license.  
For a copy, see <https://opensource.org/licenses/MIT>.*/

use std::path::{PathBuf};
use std::fs;
use gtk4::glib;

pub fn get_datadir(app_id : &str) -> Option<PathBuf> {
    let mut user_dir = glib::user_data_dir();
    let is_data = if user_dir.is_dir() {
        if let Some(dataname) = user_dir.file_name() {
            dataname.to_str() == Some("data")
        } else {
            false
        }
    } else {
        false
    };

    let parent_is_appid = if let Some(parent) = user_dir.parent() {
        if parent.is_dir() {
            if let Some(name) = parent.file_name() {
                if name.to_str() == Some(app_id) {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    // Likely a flatpak build
    if is_data && parent_is_appid {
        return Some(user_dir);
    }

    // Not likely a flatpak build. Search for appid under the returned data dir
    // (e.g. ~/.local/share).
    let entries = fs::read_dir(&user_dir).ok()?;
    for entry in entries.filter_map(|e| e.ok() ) {
        let name = entry.file_name();
        if entry.path().is_dir() && name.to_str() == Some(app_id) {
            for sub_entry in fs::read_dir(entry.path()).ok()?.filter_map(|e| e.ok() ) {
                let sub_name = sub_entry.file_name();
                if sub_entry.path().is_dir() && sub_name.to_str() == Some("data") {
                    user_dir.push(app_id);
                    user_dir.push("data");
                    return Some(user_dir);
                }
            }
            return Some(entry.path().to_owned());
        }
    }

    // At this point, $datadir/appid/data was not found. Create one and return it.
    user_dir.push(app_id);
    user_dir.push("data");
    if let Ok(_) = fs::create_dir_all(&user_dir) {
        Some(user_dir)
    } else {
        None
    }
}
