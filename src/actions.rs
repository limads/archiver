use gtk4::*;
use gtk4::prelude::*;

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

