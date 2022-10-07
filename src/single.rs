/*Copyright (c) 2022 Diego da Silva Lima. All rights reserved.

This work is licensed under the terms of the MIT license.  
For a copy, see <https://opensource.org/licenses/MIT>.*/

use gtk4::*;
use gtk4::prelude::*;
use std::fs::File;
use std::io::{Read, Write};
use std::thread;
use std::thread::JoinHandle;
use std::time::SystemTime;
use glib::signal::SignalHandlerId;
use std::convert::AsRef;
use stateful::Callbacks;
use stateful::ValuedCallbacks;
use super::{OpenDialog, SaveDialog};
use crate::FileActions;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::{Path};

#[derive(Clone, Copy)]
pub enum FileState {
    New,
    Editing,
    Open,
    CloseWindow
}

#[derive(Debug)]
pub enum SingleArchiverAction {

    // Whether to force or not
    NewRequest(bool),

    SaveRequest(Option<String>),

    SaveSuccess(String),

    SaveError(String),

    FileChanged,

    OpenRequest(String),

    // Carries path and content
    OpenSuccess(String, String),

    OpenError(String),

    RequestShowOpen,

    FileCloseRequest,

    WindowCloseRequest

}

pub struct SingleArchiver {
    send : glib::Sender<SingleArchiverAction>,
    on_open : Callbacks<(String, String)>,
    on_open_request : Callbacks<()>,
    on_new : Callbacks<()>,
    on_buffer_read_request : ValuedCallbacks<(), String>,
    on_file_changed : Callbacks<Option<String>>,
    on_save_unknown_path : Callbacks<String>,
    on_save : Callbacks<String>,
    on_close_confirm : Callbacks<String>,
    on_window_close : Callbacks<()>,
    on_show_open : Callbacks<()>,
    on_error : Callbacks<String>
}

pub trait SingleArchiverImpl : AsRef<SingleArchiver> {

    fn sender(&self) -> &glib::Sender<SingleArchiverAction> {
        &self.as_ref().send
    }

    fn connect_opened<F>(&self, f : F)
    where
        F : Fn((String, String)) + 'static
    {
        self.as_ref().on_open.bind(f);
    }

    fn connect_new<F>(&self, f : F)
    where
        F : Fn(()) + 'static
    {
        self.as_ref().on_new.bind(f);
    }

    fn connect_open_request<F>(&self, f : F)
    where
        F : Fn(()) + 'static
    {
        self.as_ref().on_open_request.bind(f);
    }

    fn connect_buffer_read_request<F>(&self, f : F)
    where
        F : Fn(())->String + 'static
    {

        // let mut on_buffer_read_request = self.as_ref().on_buffer_read_request.borrow_mut();

        // Can only connect once, since connecting multiple would assume the data
        // could be read from more than one source. But we have a single sourceview.
        assert!(self.as_ref().on_buffer_read_request.count_bounded() == 0);
        self.as_ref().on_buffer_read_request.bind(f);
    }

    // This is the first save of a new file. Perhaps rename to "save new"
    fn connect_save_unknown_path<F>(&self, f : F)
    where
        F : Fn(String)->() + 'static
    {
        self.as_ref().on_save_unknown_path.bind(f);
    }

    fn connect_error<F>(&self, f : F)
    where
        F : Fn(String)->() + 'static
    {
        self.as_ref().on_error.bind(f);
    }

    fn connect_save<F>(&self, f : F)
    where
        F : Fn(String)->() + 'static
    {
        self.as_ref().on_save.bind(f);
    }

    fn connect_close_confirm<F>(&self, f : F)
    where
        F : Fn(String) + 'static
    {
        self.as_ref().on_close_confirm.bind(f);
    }

    fn connect_file_changed<F>(&self, f : F)
    where
        F : Fn(Option<String>) + 'static
    {
        self.as_ref().on_file_changed.bind(f);
    }

    fn connect_window_close<F>(&self, f : F)
    where
        F : Fn(()) + 'static
    {
        self.as_ref().on_window_close.bind(f);
    }

    fn connect_show_open<F>(&self, f : F)
    where
        F : Fn(()) + 'static
    {
        self.as_ref().on_show_open.bind(f);
    }

}

// If file was created via "New" action, path will be None and last_saved will be None.
// If file was opened, path will be Some(path) and last_saved will be None. Every time
// the file is saved via "Save" action, last_saved will be updated and the path is
// persisted. If "Save as" is called, the last_saved AND the path are updated to the
// new path. The "SaveAs" is detected by the difference between the requested and
// actually-held paths.
#[derive(Clone, Debug, Default)]
pub struct CurrentFile {

    pub last_saved : Option<SystemTime>,

    pub path : Option<String>,

    pub just_opened : bool

}

impl CurrentFile {

    pub fn reset(&mut self) {
        self.path = None;
        self.last_saved = Some(SystemTime::now());
        self.just_opened = true;
    }

    pub fn path_or_untitled(&self) -> String {
        self.path.clone().unwrap_or(String::from("Untitled.tex"))
    }

}

impl SingleArchiver {

    pub fn new() -> Self {

        let (send, recv) = glib::MainContext::channel::<SingleArchiverAction>(glib::PRIORITY_DEFAULT);
        let on_open : Callbacks<(String, String)> = Default::default();
        let on_show_open : Callbacks<()> = Default::default();
        let on_new : Callbacks<()> = Default::default();
        let on_open_request : Callbacks<()> = Default::default();
        let on_buffer_read_request : ValuedCallbacks<(), String> = Default::default();
        let on_save_unknown_path : Callbacks<String> = Default::default();
        let on_save : Callbacks<String> = Default::default();
        let on_error : Callbacks<String> = Default::default();
        let on_close_confirm : Callbacks<String> = Default::default();
        let on_window_close : Callbacks<()> = Default::default();
        let on_file_changed : Callbacks<Option<String>> = Default::default();
        recv.attach(None, {
            let on_open = on_open.clone();
            let on_new = on_new.clone();
            let send = send.clone();
            let on_buffer_read_request = on_buffer_read_request.clone();
            let on_save_unknown_path = on_save_unknown_path.clone();
            let on_close_confirm = on_close_confirm.clone();
            let on_window_close = on_window_close.clone();
            let on_file_changed = on_file_changed.clone();
            let _on_open_request = on_open_request.clone();
            let on_save = on_save.clone();
            let on_show_open = on_show_open.clone();
            let on_error = on_error.clone();

            // Holds an action that should happen after the currently-opened file is closed.
            // This variable is updated at NewRequest, OpenRequest and WindowCloseRequest.
            let mut file_state = FileState::New;

            // Holds optional path and whether the file is saved.
            let mut curr_file : CurrentFile = Default::default();
            let mut file_open_handle : Option<JoinHandle<bool>> = None;
            let mut file_save_handle : Option<JoinHandle<bool>> = None;
            curr_file.reset();

            let mut ix = 0;
            move |action| {

                ix += 1;

                match action {

                    // To be triggered when "new" action is activated on the main menu.
                    SingleArchiverAction::NewRequest(force) => {

                        // User requested to create a new file, but the current file has unsaved changes.
                        if !force && !curr_file.last_saved.is_some() {
                            file_state = FileState::New;
                            on_close_confirm.call(curr_file.path_or_untitled());

                        // User requested to create a new file by clicking the "discard" at the toast
                        // (or there isn't a currently opened path).
                        } else {
                            curr_file.reset();
                            on_new.call(());
                        }
                    },
                    SingleArchiverAction::SaveRequest(opt_path) => {
                        if let Some(path) = opt_path {
                            let content = on_buffer_read_request.call_with_values(()).remove(0);
                            if let Some(handle) = file_save_handle.take() {
                                handle.join().unwrap();
                            }
                            file_save_handle = Some(spawn_save_file(path, content, send.clone()));
                        } else {
                            if let Some(path) = curr_file.path.clone() {
                                let content = on_buffer_read_request.call_with_values(()).remove(0);
                                if let Some(handle) = file_save_handle.take() {
                                    handle.join().unwrap();
                                }
                                file_save_handle = Some(spawn_save_file(path, content, send.clone()));
                            } else {
                                on_save_unknown_path.call(String::new());
                            }
                        }
                    },

                    // Called when the buffer changes. Ideally, when the user presses a key to
                    // insert a character. But also when the buffer is changed after a new template is
                    // loaded or a file is opened, which is why the callback is only triggered when
                    // just_opened is false.
                    SingleArchiverAction::FileChanged => {


                        // Use this decision branch to inhibit buffer changes
                        // when a new file is opened.
                        if curr_file.just_opened {
                            curr_file.just_opened = false;
                        }

                        if curr_file.last_saved.is_some() {
                            curr_file.last_saved = None;
                            on_file_changed.call(curr_file.path.clone());
                        }

                    },
                    SingleArchiverAction::SaveSuccess(path) => {
                        curr_file.path = Some(path.clone());
                        curr_file.last_saved = Some(SystemTime::now());
                        on_save.call(path.clone());
                    },
                    SingleArchiverAction::SaveError(msg) => {
                        on_error.call(msg.clone());
                    },
                    SingleArchiverAction::RequestShowOpen => {
                        if curr_file.last_saved.is_some() {
                            on_show_open.call(());
                        } else {
                            file_state = FileState::Open;
                            on_close_confirm.call(curr_file.path_or_untitled());
                        }
                    },
                    SingleArchiverAction::OpenRequest(path) => {

                        // User tried to open an already-opened file. Ignore the request in this case.
                        if let Some(curr_path) = &curr_file.path {
                            if &curr_path[..] == path {
                                return Continue(true);
                            }
                        }
    
                        if let Some(handle) = file_open_handle.take() {
                            handle.join().unwrap();
                        }
                        file_open_handle = Some(spawn_open_file(path, send.clone()));

                        // Just opened should be set here (before the confirmation of the open thread)
                        // because the on_open
                        // curr_file.just_opened = true;
                    },
                    SingleArchiverAction::OpenSuccess(path, content) => {

                        // It is critical that just_opened is set to true before calling the on_open,
                        // because we must ignore the change to the sourceview buffer.
                        curr_file.just_opened = true;
                        curr_file.path = Some(path.clone());
                        curr_file.last_saved = Some(SystemTime::now());

                        on_open.call((path.clone(), content.clone()));

                    },

                    SingleArchiverAction::OpenError(e) => {
                        on_error.call(e.clone());
                    },

                    // Triggered when the user choses to close an unsaved file at the toast.
                    SingleArchiverAction::FileCloseRequest => {
                        curr_file.reset();
                        match file_state {
                            FileState::New => {
                                on_new.call(());
                                curr_file.just_opened = true;
                            },
                            FileState::Open => {
                                on_show_open.call(());
                                curr_file.just_opened = true;
                            },
                            FileState::CloseWindow => {
                                on_window_close.call(());
                            },
                            FileState::Editing => {

                            }
                        }
                    },
                    SingleArchiverAction::WindowCloseRequest => {
                        if !curr_file.last_saved.is_some() {
                            file_state = FileState::CloseWindow;
                            on_close_confirm.call(curr_file.path_or_untitled());
                        } else {
                            on_window_close.call(());
                        }
                    }
                }
                Continue(true)
            }
        });
        Self {
            on_open,
            send,
            on_save_unknown_path,
            on_buffer_read_request,
            on_close_confirm,
            on_window_close,
            on_new,
            on_save,
            on_file_changed,
            on_open_request,
            on_show_open,
            on_error
        }
    }

}

/// Spawns thread to open a filesystem file. The result of the operation will
/// be sent back to the main thread via the send glib channel.
pub fn spawn_open_file(path : String, send : glib::Sender<SingleArchiverAction>) -> JoinHandle<bool> {
    thread::spawn(move || {
    
        if !Path::new(&path[..]).is_absolute() {
            send.send(SingleArchiverAction::SaveError(String::from("Using non-absolute path")))
                .unwrap_or_else(super::log_err);
            return false;
        }
        
        match File::open(&path) {
            Ok(mut f) => {
                let mut content = String::new();
                match f.read_to_string(&mut content) {
                    Ok(_) => {
                        if let Err(e) = send.send(SingleArchiverAction::OpenSuccess(path.to_string(), content)) {
                            eprintln!("{}", e);
                        }
                        true
                    },
                    Err(e) => {
                        if let Err(e) = send.send(SingleArchiverAction::OpenError(format!("{}", e ))) {
                            eprintln!("{}", e);
                        }
                        false
                    }
                }
            },
            Err(e) => {
                if let Err(e) = send.send(SingleArchiverAction::OpenError(format!("{}", e ))) {
                    eprintln!("{}", e);
                }
                false
            }
        }
    })
}

pub fn spawn_save_file(
    path : String,
    content : String,
    send : glib::Sender<SingleArchiverAction>
) -> JoinHandle<bool> {
    thread::spawn(move || {

        if !Path::new(&path[..]).is_absolute() {
            send.send(SingleArchiverAction::SaveError(String::from("Using non-absolute path")))
                .unwrap_or_else(super::log_err);
            return false;
        }
        
        if Path::new(&path[..]).is_dir() {
            send.send(SingleArchiverAction::SaveError(String::from("Tried to save file to directory path")))
                .unwrap_or_else(super::log_err);
            return false;
        }

        match File::create(&path) {
            Ok(mut f) => {
                match f.write_all(content.as_bytes()) {
                    Ok(_) => {
                        send.send(SingleArchiverAction::SaveSuccess(path))
                            .unwrap_or_else(super::log_err);
                        true
                    },
                    Err(e) => {
                        send.send(SingleArchiverAction::SaveError(format!("{}",e )))
                            .unwrap_or_else(super::log_err);
                        false
                    }
                }
            }
            Err(e) => {
                send.send(SingleArchiverAction::SaveError(format!("{}",e )))
                    .unwrap_or_else(super::log_err);
                false
            }
        }
    })
}

pub fn connect_manager_with_open_dialog(send : &glib::Sender<SingleArchiverAction>, dialog : &OpenDialog) {
    let send = send.clone();
    dialog.dialog.connect_response(move |dialog, resp| {
        match resp {
            ResponseType::Accept => {
                if let Some(path) = dialog.file().and_then(|f| f.path() ) {
                    send.send(SingleArchiverAction::OpenRequest(path.to_str().unwrap().to_string())).unwrap();
                }
            },
            _ => { }
        }
    });
}

pub fn connect_manager_with_save_dialog(send : &glib::Sender<SingleArchiverAction>, dialog : &SaveDialog) {
    let send = send.clone();
    dialog.dialog.connect_response(move |dialog, resp| {
        match resp {
            ResponseType::Accept => {
                if let Some(path) = dialog.file().and_then(|f| f.path() ) {
                    send.send(SingleArchiverAction::SaveRequest(Some(path.to_str().unwrap().to_string()))).unwrap();
                }
            },
            _ => { }
        }
    });
}

pub fn connect_manager_with_editor(
    send : &glib::Sender<SingleArchiverAction>,
    view : &sourceview5::View,
    ignore_file_save_action : &gio::SimpleAction
) -> SignalHandlerId {
    ignore_file_save_action.connect_activate({
        let send = send.clone();
        move |_action, _param| {
            send.send(SingleArchiverAction::FileCloseRequest).unwrap();
        }
    });
    view.buffer().connect_changed({
        let send = send.clone();
        move |_buf| {
            send.send(SingleArchiverAction::FileChanged).unwrap();
        }
    })
}

// This is a reaction of the manager to changes in the window
pub fn connect_manager_responds_window(send : &glib::Sender<SingleArchiverAction>, window : &ApplicationWindow) {
    let send = send.clone();
    window.connect_close_request(move |_win| {
        send.send(SingleArchiverAction::WindowCloseRequest).unwrap();
        glib::signal::Inhibit(true)
    });
}

// This is a reaction of the window to changes in the manager
pub fn connect_manager_with_app_window_and_actions<A>(
    manager : &A,
    window : &ApplicationWindow,
    actions : &FileActions,
    extension : &'static str
)
where
    A : AsRef<SingleArchiver> + SingleArchiverImpl
{
    let win = window.clone();
    manager.connect_window_close(move |_| {
        win.destroy();
    });
    manager.connect_opened({
        let action_save = actions.save.clone();
        let action_save_as = actions.save_as.clone();
        let window = window.clone();
        move |(path, _)| {
            action_save.set_enabled(true);
            action_save_as.set_enabled(true);
            window.set_title(Some(&path));
        }
    });
    manager.connect_open_request({
        let open_action = actions.open.clone();
        move |_| {
            open_action.activate(None);
        }
    });
    manager.connect_save({
        let window = window.clone();
        move |path| {
            window.set_title(Some(&path));
        }
    });
    manager.connect_file_changed({
        let window = window.clone();
        move |opt_path| {
            if let Some(path) = opt_path {
                window.set_title(Some(&format!("{}*", path)));
            } else {
                window.set_title(Some(&format!("Untitled.{}*", extension)));
            }
        }
    });
}

pub fn connect_manager_with_file_actions(
    // manager : &FileManager,
    actions : &super::FileActions,
    send : &glib::Sender<SingleArchiverAction>,
    open_dialog : &OpenDialog
) {
    actions.new.connect_activate({
        let send = send.clone();
        move |_,_| {
            send.send(SingleArchiverAction::NewRequest(false)).unwrap();
        }
    });
    actions.save.connect_activate({
        let send = send.clone();
        move |_,_| {
            send.send(SingleArchiverAction::SaveRequest(None))
                .unwrap_or_else(super::log_err);
        }
    });
    let _open_dialog = open_dialog.clone();
    actions.open.connect_activate({
        let send = send.clone();
        move |_,_| {
            send.send(SingleArchiverAction::RequestShowOpen)
                .unwrap_or_else(super::log_err);
        }
    });
}

pub fn connect_manager_to_editor<A>(
    manager : &A,
    view : &sourceview5::View,
    buf_change_handler : &Rc<RefCell<Option<SignalHandlerId>>>
)
where
    A : AsRef<SingleArchiver> + SingleArchiverImpl
{
    manager.connect_opened({
        let view = view.clone();
        let change_handler = buf_change_handler.clone();
        move |(_path, content)| {
            let handler_guard = change_handler.borrow();
            let change_handler = handler_guard.as_ref().unwrap();
            view.buffer().block_signal(&change_handler);
            view.buffer().set_text(&content);
            view.buffer().unblock_signal(&change_handler);
        }
    });
    manager.connect_buffer_read_request({
        let view = view.clone();
        move |_| -> String {
            let buffer = view.buffer();
            buffer.text(
                &buffer.start_iter(),
                &buffer.end_iter(),
                true
            ).to_string()
        }
    });
}

