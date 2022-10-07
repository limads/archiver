/*Copyright (c) 2022 Diego da Silva Lima. All rights reserved.

This work is licensed under the terms of the MIT license.  
For a copy, see <https://opensource.org/licenses/MIT>.*/


use gtk4::*;

#[derive(Debug, Clone)]
pub struct FileActions {
    pub new : gio::SimpleAction,
    pub open : gio::SimpleAction,
    pub save : gio::SimpleAction,
    pub save_as : gio::SimpleAction
}

impl FileActions {

    pub fn new() -> Self {
        let new = gio::SimpleAction::new("new_file", None);
        let open = gio::SimpleAction::new("open_file", None);
        let save = gio::SimpleAction::new("save_file", None);
        let save_as = gio::SimpleAction::new("save_as_file", None);
        Self { new, open, save, save_as }
    }

}

