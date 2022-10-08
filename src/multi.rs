/*Copyright (c) 2022 Diego da Silva Lima. All rights reserved.

This work is licensed under the terms of the MIT license.  
For a copy, see <https://opensource.org/licenses/MIT>.*/

use std::thread;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path};
use std::thread::JoinHandle;
use serde::{Serialize, Deserialize};
// use chrono::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use gtk4::glib;
use stateful::{Callbacks, ValuedCallbacks, Inherit};
use std::time::SystemTime;

pub trait MultiArchiverImpl : Inherit<Parent = MultiArchiver> {

    fn final_state(&self) -> Rc<RefCell<FinalState>> {
        self.parent().final_state.clone()
    }

    fn add_files(&self, files : &[OpenedFile]) {
        for f in files.iter() {
            self.parent().send.send(MultiArchiverAction::Add(f.clone()))
                .unwrap_or_else(super::log_err);
        }
    }

    fn sender(&self) -> &glib::Sender<MultiArchiverAction> {
        &self.parent().send
    }

    fn connect_new<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.parent().on_new.bind(f);
    }

    // When the user requested to open a file that was already opened. Gives
    // the client a chance to do someting, such as making the file view receive
    // the focs.
    fn connect_reopen<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.parent().on_reopen.bind(f);
    }

    fn connect_added<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.parent().on_added.bind(f);
    }

    fn connect_selected<F>(&self, f : F)
    where
        F : Fn(Option<OpenedFile>) + 'static
    {
        self.parent().on_selected.bind(f);
    }

    fn connect_opened<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.parent().on_open.bind(f);
    }

    fn connect_closed<F>(&self, f : F)
    where
        F : Fn((OpenedFile, usize)) + 'static
    {
        self.parent().on_file_closed.bind(f);
    }

    fn connect_close_confirm<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.parent().on_close_confirm.bind(f);
    }

    fn connect_file_changed<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.parent().on_file_changed.bind(f);
    }

    fn connect_file_persisted<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.parent().on_file_persisted.bind(f);
    }

    fn connect_error<F>(&self, f : F)
    where
        F : Fn(String) + 'static
    {
        self.parent().on_error.bind(f);
    }

    fn connect_on_active_text_changed<F>(&self, f : F)
    where
        F : Fn(Option<String>) + 'static
    {
        self.parent().on_active_text_changed.bind(f);
    }

    fn connect_window_close<F>(&self, f : F)
    where
        F : Fn(()) + 'static
    {
        self.parent().on_window_close.bind(f);
    }

    fn connect_save_unknown_path<F>(&self, f : F)
    where
        F : Fn(String) + 'static
    {
        self.parent().on_save_unknown_path.bind(f);
    }

    fn connect_buffer_read_request<F>(&self, f : F)
    where
        F : Fn(usize)->String + 'static
    {
        self.parent().on_buffer_read_request.bind(f);
    }

    fn connect_name_changed<F>(&self, f : F)
    where
        F : Fn((usize, String)) + 'static
    {
        self.parent().on_name_changed.bind(f);
    }

}

#[derive(Debug, Clone)]
pub struct FinalState {
    pub recent : Vec<OpenedFile>,
    pub files : Vec<OpenedFile>
}

#[derive(Debug, Clone)]
pub enum MultiArchiverAction {

    OpenRequest(String),
    
    OpenRelativeRequest(String),
    
    SetPrefix(Option<String>),

    OpenSuccess(OpenedFile),

    // Represents an addition to the recent script file list (not necessarily opened).
    Add(OpenedFile),

    OpenError(String),

    // OpenFailure(String),

    // File position and whether the request is "forced" (i.e. asks for user confirmation).
    CloseRequest(usize, bool),

    // CloseConfirm(usize),

    SaveRequest(Option<String>),

    SaveSuccess(usize, String),

    SaveError(String),

    // Opened(String),

    // Closed(String),

    NewRequest,

    // ActiveTextChanged(Option<String>),

    WindowCloseRequest,

    SetSaved(usize, bool),

    Select(Option<usize>),

}

pub struct MultiArchiver {

    final_state : Rc<RefCell<FinalState>>,

    send : glib::Sender<MultiArchiverAction>,

    on_open : Callbacks<OpenedFile>,

    on_error : Callbacks<String>,

    // on_save : Callbacks<OpenedFile>,

    on_reopen : Callbacks<OpenedFile>,

    on_save_unknown_path : Callbacks<String>,

    on_file_changed : Callbacks<OpenedFile>,

    on_file_persisted : Callbacks<OpenedFile>,

    on_active_text_changed : Callbacks<Option<String>>,

    // When user clicks new action
    on_new : Callbacks<OpenedFile>,

    // Contains the index of the old closed file and the number of remaining files.
    on_file_closed : Callbacks<(OpenedFile, usize)>,

    on_close_confirm : Callbacks<OpenedFile>,

    on_window_close : Callbacks<()>,

    on_buffer_read_request : ValuedCallbacks<usize, String>,

    on_selected : Callbacks<Option<OpenedFile>>,

    // Called when file goes from untitled to having a name.
    on_name_changed : Callbacks<(usize, String)>,

    // When the user state is being updated
    on_added : Callbacks<OpenedFile>

}

// Some SQL files (e.g. generated by pg_dump) are too big for gtksourceview.
// Limiting the file size prevents the application from freezing.
const MAX_FILE_SIZE : usize = 5_000_000;

impl MultiArchiver {

    pub fn final_state(&self) -> FinalState {
        self.final_state.borrow().clone()
    }

    pub fn sender(&self) -> &glib::Sender<MultiArchiverAction> {
        &self.send
    }

    pub fn new(extension : String) -> Self {
        let final_state = Rc::new(RefCell::new(FinalState { recent : Vec::new(), files : Vec::new() }));
        let (send, recv) = glib::MainContext::channel::<MultiArchiverAction>(glib::PRIORITY_DEFAULT);
        let on_open : Callbacks<OpenedFile> = Default::default();
        let on_new : Callbacks<OpenedFile> = Default::default();
        // let on_save : Callbacks<OpenedFile> = Default::default();
        let on_file_changed : Callbacks<OpenedFile> = Default::default();
        let on_file_persisted : Callbacks<OpenedFile> = Default::default();
        let on_reopen : Callbacks<OpenedFile> = Default::default();
        let on_selected : Callbacks<Option<OpenedFile>> = Default::default();
        let on_file_closed : Callbacks<(OpenedFile, usize)> = Default::default();
        let on_active_text_changed : Callbacks<Option<String>> = Default::default();
        let on_close_confirm : Callbacks<OpenedFile> = Default::default();
        let on_window_close : Callbacks<()> = Default::default();
        let on_save_unknown_path : Callbacks<String> = Default::default();
        let on_buffer_read_request : ValuedCallbacks<usize, String> = Default::default();
        let on_name_changed : Callbacks<(usize, String)> = Default::default();
        let on_error : Callbacks<String> = Default::default();
        let on_added : Callbacks<OpenedFile> = Default::default();

        // Holds the files opened at the editor the user sees on the side panel
        let mut files : Vec<OpenedFile> = Vec::new();

        // Holds the files shown on the recent script list before the editor is opened. The files
        // are loaded on startup. If the user saves or opens any files not already on this list,
        // the list is updated. This list is sent to the final_state just before the application
        // closes.
        let mut recent_files : Vec<OpenedFile> = Vec::new();

        let mut selected : Option<usize> = None;

        let mut win_close_request = false;
        recv.attach(None, {
            let send = send.clone();
            let (on_open, on_new, /*_on_save,*/ on_selected, on_file_closed, on_close_confirm, on_file_changed, on_file_persisted, on_reopen) = (
                on_open.clone(),
                on_new.clone(),
                // on_save.clone(),
                on_selected.clone(),
                on_file_closed.clone(),
                on_close_confirm.clone(),
                on_file_changed.clone(),
                on_file_persisted.clone(),
                on_reopen.clone()
            );
            let (_on_active_text_changed, on_window_close, on_buffer_read_request, on_save_unknown_path) = (
                on_active_text_changed.clone(),
                on_window_close.clone(),
                on_buffer_read_request.clone(),
                on_save_unknown_path.clone()
            );
            let on_added = on_added.clone();
            let on_name_changed = on_name_changed.clone();
            let on_error = on_error.clone();
            let mut file_open_handle : Option<JoinHandle<bool>> = None;
            let mut file_save_handle : Option<JoinHandle<bool>> = None;

            let mut last_closed_file : Option<OpenedFile> = None;
            let final_state = final_state.clone();
            
            // If set, any file operations are only done if the path satisfies
            // this prefix (e.g. multiarchiver does not touch anything outside
            // /home/user/myproject if prefix is set to this value.
            let mut prefix : Option<String> = None;

            move |action| {

                match action {

                    // When user clicks "new file"
                    MultiArchiverAction::NewRequest => {
                        if files.len() == 16 {
                            send.send(MultiArchiverAction::OpenError(format!("Maximum number of files opened"))).unwrap();
                            return glib::source::Continue(true);
                        }
                        let n_untitled = files.iter().filter(|f| f.name.starts_with("Untitled") )
                            .last()
                            .map(|f| f.name.split(" ").nth(1).unwrap().trim_end_matches(&format!(".{}", extension)).parse::<usize>().unwrap() )
                            .unwrap_or(0);
                        let new_file = OpenedFile {
                            path : None,
                            name : format!("Untitled {}.{}", n_untitled + 1, extension),
                            saved : true,
                            content : None,
                            index : files.len(),
                            dt : Some(SystemTime::now())
                        };
                        files.push(new_file.clone());
                        on_new.call(new_file);
                    },

                    // When the user state is being updated
                    MultiArchiverAction::Add(file) => {
                        recent_files.push(file.clone());
                        on_added.call(file);
                    },
                    MultiArchiverAction::OpenRelativeRequest(rel_path) => {
                        if let Some(pr) = &prefix {
                            // TODO this will break if file path is wrt workspace, since diagnostic
                            // messages are relative to whole workspace.
                            let abs = Path::new(pr).to_path_buf().join(rel_path);
                            send.send(MultiArchiverAction::OpenRequest(abs.display().to_string())).unwrap();                            
                        } else {
                            send.send(MultiArchiverAction::OpenError(format!("No path prefix set"))).unwrap();
                        }
                    },
                    MultiArchiverAction::OpenRequest(path) => {

                        if let Some(pr) = &prefix {
                            if !path.starts_with(pr) {
                                send.send(MultiArchiverAction::OpenError(format!("Cannot open file outside prefix {}", pr))).unwrap();
                                return glib::source::Continue(true);
                            }
                        }
                        
                        if let Some(already_opened) = files.iter().find(|f| f.path.as_ref().map(|p| &p[..] == &path[..] ).unwrap_or(false) ) {

                            // send.send(MultiArchiverAction::OpenError(format!("File already opened"))).unwrap();
                            on_reopen.call(already_opened.clone());
                            return glib::source::Continue(true);
                        }

                        if files.len() == 16 {
                            send.send(MultiArchiverAction::OpenError(format!("File list limit reached"))).unwrap();
                            return glib::source::Continue(true);
                        }

                        // We could have a problem if the user attempts to open
                        // two files in extremely quick succession, and/or for any reason opening the first
                        // file takes too long (e.g. a busy hard drive). If a second file is opened
                        // before the first file opening thread ends, the two files would receive the
                        // same index, since the file index is moved when the thead is spawned.
                        // The ocurrence should be rare enough to justify blocking the main thread here.
                        if let Some(handle) = file_open_handle.take() {
                            handle.join().unwrap();
                        }

                        file_open_handle = Some(spawn_open_file(send.clone(), path, files.len()));
                    },
                    MultiArchiverAction::CloseRequest(ix, force) => {

                        if ix >= files.len() {
                            eprintln!("Invalid file index at close request: {}", ix);
                            return glib::source::Continue(true);
                        }
                        
                        // This force=true branch will be hit by a request from the toast button
                        // clicked when the user wants to ignore an unsaved file. If win_close_request=true,
                        // the action originated from a application window close. If win_close_request=false,
                        // the action originated from a file list item close.
                        if force {
                            let closed_file = remove_file(&mut files, ix);
                            assert!(closed_file.index == ix);
                            last_closed_file = Some(closed_file.clone());
                            let n = files.len();
                            on_file_closed.call((closed_file, n));
                            if win_close_request {
                                on_window_close.call(());
                                win_close_request = false;
                            }
                        } else {
                            if files[ix].saved {
                                let closed_file = remove_file(&mut files, ix);
                                assert!(closed_file.index == ix);
                                last_closed_file = Some(closed_file.clone());
                                let n = files.len();
                                on_file_closed.call((closed_file, n));
                            } else {
                                on_close_confirm.call(files[ix].clone());
                            }
                        }
                        final_state.replace(FinalState { recent : recent_files.clone(), files : files.clone() });
                    },
                    MultiArchiverAction::SaveRequest(opt_path) => {
                        if let Some(ix) = selected {
                            if let Some(path) = opt_path {
                            
                                if let Some(pr) = &prefix {
                                    if !path.starts_with(pr) {
                                        send.send(MultiArchiverAction::OpenError(format!("Cannot save file outside prefix {}", pr))).unwrap();
                                        return glib::source::Continue(true);
                                    }
                                }
                                
                                let content = on_buffer_read_request.call_with_values(ix).remove(0);
                                if let Some(handle) = file_save_handle.take() {
                                    handle.join().unwrap();
                                }
                                file_save_handle = Some(spawn_save_file(path, ix, content, send.clone()));
                            } else {
                                if let Some(path) = files[ix].path.clone() {
                                
                                    if let Some(pr) = &prefix {
                                        if !path.starts_with(pr) {
                                            send.send(MultiArchiverAction::OpenError(format!("Cannot save file outside prefix {}", pr))).unwrap();
                                            return glib::source::Continue(true);
                                        }
                                    }
                                    
                                    let content = on_buffer_read_request.call_with_values(ix).remove(0);
                                    if let Some(handle) = file_save_handle.take() {
                                        handle.join().unwrap();
                                    }
                                    file_save_handle = Some(spawn_save_file(path, ix, content, send.clone()));
                                } else {
                                    on_save_unknown_path.call(files[ix].name.clone());
                                }
                            }
                        } else {
                            panic!("No file selected");
                        }
                    },
                    MultiArchiverAction::SaveSuccess(ix, path) => {
                    
                        if ix >= files.len() {
                            eprintln!("Invalid file index after save success: {}", ix);
                            return glib::source::Continue(true);
                        }
                        
                        if files[ix].name.starts_with("Untitled") {
                            files[ix].name = path.clone();
                            files[ix].path = Some(path.clone());
                            on_name_changed.call((ix, path.clone()));

                            if recent_files.iter().find(|f| &f.path.as_ref().unwrap()[..] == &path[..] ).is_none() {
                                recent_files.push(files[ix].clone());
                            }
                        }
                        send.send(MultiArchiverAction::SetSaved(ix, true))
                            .unwrap_or_else(super::log_err);
                    },
                    MultiArchiverAction::SaveError(e) => {
                        on_error.call(e);
                    },
                    MultiArchiverAction::SetSaved(ix, saved) => {

                        if ix >= files.len() {
                            eprintln!("Invalid file index at set saved: {}", ix);
                            return glib::source::Continue(true);
                        }
                        
                        // SetSaved will be called when a buffer is cleared after a file is closed,
                        // so we just ignore the call in this case, since the file won't be at the
                        // buffer anymore (impl React<QueriesEditor> for MultiArchiver).
                        if last_closed_file.clone().map(|f| f.index == ix ).unwrap_or(false) {
                            last_closed_file = None;
                            return glib::source::Continue(true);
                        }

                        if saved {
                            files[ix].saved = true;
                            on_file_persisted.call(files[ix].clone());
                        } else {
                        
                            // TODO thread 'main' panicked at 'index out of bounds: the len is 1 
                            // but the index is 1', /home/diego/.cargo/registry/src/github.com-1ecc6299db9ec823/filecase-0.1.1/src/multi.rs:492:32

                            if files[ix].saved {
                                files[ix].saved = false;
                                on_file_changed.call(files[ix].clone());
                            }
                        }
                    },
                    MultiArchiverAction::OpenSuccess(file) => {
                        files.push(file.clone());
                        on_open.call(file.clone());
                        send.send(MultiArchiverAction::SetSaved(file.index, true))
                            .unwrap_or_else(super::log_err);

                        if recent_files.iter().find(|f| &f.path.as_ref().unwrap()[..] == &file.path.as_ref().unwrap()[..] ).is_none() {
                            recent_files.push(file.clone());
                        }
                    },
                    MultiArchiverAction::OpenError(msg) => {
                        on_error.call(msg.clone());
                    },
                    MultiArchiverAction::SetPrefix(opt_path) => {
                        prefix = opt_path;
                    },
                    MultiArchiverAction::Select(opt_ix) => {
                        
                        if let Some(ix) = opt_ix {
                            if ix >= files.len() {
                                eprintln!("Invalid file index at selection: {}", ix);
                                return glib::source::Continue(true);
                            }
                        }
                        
                        selected = opt_ix;
                        on_selected.call(opt_ix.map(|ix| files[ix].clone() ));
                    },
                    MultiArchiverAction::WindowCloseRequest => {
                        if let Some(file) = files.iter().filter(|file| !file.saved ).next() {
                            on_close_confirm.call(file.clone());
                            win_close_request = true;
                        } else {
                            on_window_close.call(());
                        }
                        final_state.replace(FinalState { recent : recent_files.clone(), files : files.clone() });
                    }
                }
                glib::source::Continue(true)
            }
        });

        // File change watch thread
        /*let (tx, rx) = channel();
        let mut watcher = notify::watcher(tx, Duration::from_secs(5)).unwrap();
        thread::spawn({
            let sender = sender.clone();
            move|| {
                loop {
                    match rx.recv() {
                        Ok(event) => {
                            /*match event.op {
                                Ok(notify::op::Op::WRITE)
                                Ok(notify::op::Op::CREATE)
                                Ok(notify::op::Op::RENAME)
                                Ok(notify::op::Op::CHMOD)
                                Ok(notify::op::Op::REMOVE)
                            }*/
                        },
                       Err(_) => { },
                    }
                }
            }
        });*/
        Self {
            on_open,
            on_new,
            send,
            on_selected,
            on_file_closed,
            on_close_confirm,
            on_file_changed,
            on_file_persisted,
            on_active_text_changed,
            on_window_close,
            on_buffer_read_request,
            on_save_unknown_path,
            on_name_changed,
            on_error,
            on_added,
            on_reopen,
            final_state
        }
    }

}

// To save file...
/*if let Some(path) = file.path {
        if Self::save_file(&path, self.get_text()) {
            self.file_list.mark_current_saved();
            println!("Content written into file");
        } else {
            println!("Unable to save file");
        }
    } else {
        self.sql_save_dialog.set_filename(&file.name);
        self.sql_save_dialog.run();
        self.sql_save_dialog.hide();
    }
}
*/

// TO open file..
// view.get_buffer().map(|buf| buf.set_text(&content) );

// To get text...
/*
pub fn get_text(&self) -> String {
    if let Some(buffer) = self.view.borrow().get_buffer() {
        let txt = buffer.get_text(
            &buffer.get_start_iter(),
            &buffer.get_end_iter(),
            true
        ).unwrap();
        txt.to_string()
    } else {
        panic!("Unable to retrieve text buffer");
    }
} */

fn remove_file(files : &mut Vec<OpenedFile>, ix : usize) -> OpenedFile {
    files[(ix+1)..].iter_mut().for_each(|f| f.index -= 1 );
    files.remove(ix)
}

fn spawn_save_file(
    path : String,
    index : usize,
    content : String,
    send : glib::Sender<MultiArchiverAction>
) -> JoinHandle<bool> {
    thread::spawn(move || {
    
        if !Path::new(&path[..]).is_absolute() {
            send.send(MultiArchiverAction::SaveError(String::from("Using non-absolute path")))
                .unwrap_or_else(super::log_err);
            return false;
        }
        
        if Path::new(&path[..]).is_dir() {
            send.send(MultiArchiverAction::SaveError(String::from("Tried to save file to directory path")))
                .unwrap_or_else(super::log_err);
            return false;
        }
        
        match File::create(&path) {
            Ok(mut f) => {
                match f.write_all(content.as_bytes()) {
                    Ok(_) => {
                        send.send(MultiArchiverAction::SaveSuccess(index, path))
                            .unwrap_or_else(super::log_err);
                        true
                    },
                    Err(e) => {
                        send.send(MultiArchiverAction::SaveError(format!("{}", e)))
                            .unwrap_or_else(super::log_err);
                        false
                    }
                }
            },
            Err(e) => {
                send.send(MultiArchiverAction::SaveError(format!("{}", e)))
                    .unwrap_or_else(super::log_err);
                false
            }
        }
    })
}

fn spawn_open_file(send : glib::Sender<MultiArchiverAction>, path : String, n_files : usize) -> JoinHandle<bool> {
    thread::spawn(move || {
    
        if !Path::new(&path[..]).is_absolute() {
            send.send(MultiArchiverAction::SaveError(String::from("Using non-absolute path")))
                .unwrap_or_else(super::log_err);
            return false;
        }
        
        match File::open(&path) {
            Ok(mut f) => {
                let mut content = String::new();
                if let Err(e) = f.read_to_string(&mut content) {
                    send.send(MultiArchiverAction::OpenError(format!("{}", e)))
                        .unwrap_or_else(super::log_err);
                }

                if content.len() > MAX_FILE_SIZE {
                    send.send(MultiArchiverAction::OpenError(format!("File extrapolates maximum size"))).unwrap();
                    return false;
                }

                let new_file = OpenedFile {
                    path : Some(path.clone()),
                    name : path.clone(),
                    saved : true,
                    content : Some(content),
                    index : n_files,
                    dt : Some(SystemTime::now())
                };
                send.send(MultiArchiverAction::OpenSuccess(new_file)).unwrap();
                true
            },
            Err(e) => {
                send.send(MultiArchiverAction::OpenError(format!("{}", e))).unwrap();
                false
            }
        }
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenedFile {
    pub name : String,
    pub path : Option<String>,
    pub content : Option<String>,
    pub saved : bool,
    pub dt : Option<SystemTime>,
    pub index : usize
}



