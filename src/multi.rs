use std::boxed;
use std::thread;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use notify::{self, Watcher};
use std::sync::mpsc;
use std::time::Duration;
use std::thread::JoinHandle;
use serde::{Serialize, Deserialize};
use chrono::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::convert::AsRef;
use gtk4::glib;
use stateful::{Callbacks, ValuedCallbacks};

pub trait MultiArchiverImpl : AsRef<MultiArchiver> {

    fn final_state(&self) -> Rc<RefCell<Vec<OpenedFile>>> {
        self.as_ref().final_state.clone()
    }

    fn add_files(&self, files : &[OpenedFile]) {
        for f in files.iter() {
            self.as_ref().send.send(MultiArchiverAction::Add(f.clone()));
        }
    }

    fn sender(&self) -> &glib::Sender<MultiArchiverAction> {
        &self.as_ref().send
    }

    fn connect_new<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.as_ref().on_new.bind(f);
    }

    fn connect_added<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.as_ref().on_added.bind(f);
    }

    fn connect_selected<F>(&self, f : F)
    where
        F : Fn(Option<OpenedFile>) + 'static
    {
        self.as_ref().on_selected.bind(f);
    }

    fn connect_opened<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.as_ref().on_open.bind(f);
    }

    fn connect_closed<F>(&self, f : F)
    where
        F : Fn((usize, usize)) + 'static
    {
        self.as_ref().on_file_closed.bind(f);
    }

    fn connect_close_confirm<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.as_ref().on_close_confirm.bind(f);
    }

    fn connect_file_changed<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.as_ref().on_file_changed.bind(f);
    }

    fn connect_file_persisted<F>(&self, f : F)
    where
        F : Fn(OpenedFile) + 'static
    {
        self.as_ref().on_file_persisted.bind(f);
    }

    fn connect_error<F>(&self, f : F)
    where
        F : Fn(String) + 'static
    {
        self.as_ref().on_error.bind(f);
    }

    fn connect_on_active_text_changed<F>(&self, f : F)
    where
        F : Fn(Option<String>) + 'static
    {
        self.as_ref().on_active_text_changed.bind(f);
    }

    fn connect_window_close<F>(&self, f : F)
    where
        F : Fn(()) + 'static
    {
        self.as_ref().on_window_close.bind(f);
    }

    fn connect_save_unknown_path<F>(&self, f : F)
    where
        F : Fn(String) + 'static
    {
        self.as_ref().on_save_unknown_path.bind(f);
    }

    fn connect_buffer_read_request<F>(&self, f : F)
    where
        F : Fn(usize)->String + 'static
    {
        self.as_ref().on_buffer_read_request.bind(f);
    }

    fn connect_name_changed<F>(&self, f : F)
    where
        F : Fn((usize, String)) + 'static
    {
        self.as_ref().on_name_changed.bind(f);
    }

}

pub enum MultiArchiverAction {

    OpenRequest(String),

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

    final_state : Rc<RefCell<Vec<OpenedFile>>>,

    send : glib::Sender<MultiArchiverAction>,

    on_open : Callbacks<OpenedFile>,

    on_error : Callbacks<String>,

    on_save : Callbacks<OpenedFile>,

    on_save_unknown_path : Callbacks<String>,

    on_file_changed : Callbacks<OpenedFile>,

    on_file_persisted : Callbacks<OpenedFile>,

    on_active_text_changed : Callbacks<Option<String>>,

    on_new : Callbacks<OpenedFile>,

    on_file_closed : Callbacks<(usize, usize)>,

    on_close_confirm : Callbacks<OpenedFile>,

    on_window_close : Callbacks<()>,

    on_buffer_read_request : ValuedCallbacks<usize, String>,

    on_selected : Callbacks<Option<OpenedFile>>,

    on_name_changed : Callbacks<(usize, String)>,

    on_added : Callbacks<OpenedFile>

}

// Some SQL files (e.g. generated by pg_dump) are too big for gtksourceview.
// Limiting the file size prevents the application from freezing.
const MAX_FILE_SIZE : usize = 5_000_000;

impl MultiArchiver {

    pub fn new() -> Self {
        let final_state = Rc::new(RefCell::new(Vec::new()));
        let (send, recv) = glib::MainContext::channel::<MultiArchiverAction>(glib::PRIORITY_DEFAULT);
        let on_open : Callbacks<OpenedFile> = Default::default();
        let on_new : Callbacks<OpenedFile> = Default::default();
        let on_save : Callbacks<OpenedFile> = Default::default();
        let on_file_changed : Callbacks<OpenedFile> = Default::default();
        let on_file_persisted : Callbacks<OpenedFile> = Default::default();
        let on_selected : Callbacks<Option<OpenedFile>> = Default::default();
        let on_file_closed : Callbacks<(usize, usize)> = Default::default();
        let on_active_text_changed : Callbacks<Option<String>> = Default::default();
        let on_close_confirm : Callbacks<OpenedFile> = Default::default();
        let on_window_close : Callbacks<()> = Default::default();
        let on_save_unknown_path : Callbacks<String> = Default::default();
        let on_buffer_read_request : ValuedCallbacks<usize, String> = Default::default();
        let on_name_changed : Callbacks<(usize, String)> = Default::default();
        let on_error : Callbacks<String> = Default::default();
        let on_added : Callbacks<OpenedFile> = Default::default();

        // Holds the files opened at the editor the user seeds on the side panel
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
            let (on_open, on_new, on_save, on_selected, on_file_closed, on_close_confirm, on_file_changed, on_file_persisted) = (
                on_open.clone(),
                on_new.clone(),
                on_save.clone(),
                on_selected.clone(),
                on_file_closed.clone(),
                on_close_confirm.clone(),
                on_file_changed.clone(),
                on_file_persisted.clone()
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
            let mut file_open_handle : Option<JoinHandle<()>> = None;

            let mut last_closed_file : Option<OpenedFile> = None;
            let final_state = final_state.clone();

            move |action| {

                // println!("Current files = {:?}", files);
                match action {
                    MultiArchiverAction::NewRequest => {
                        if files.len() == 16 {
                            send.send(MultiArchiverAction::OpenError(format!("Maximum number of files opened"))).unwrap();
                            return glib::source::Continue(true);
                        }
                        let n_untitled = files.iter().filter(|f| f.name.starts_with("Untitled") )
                            .last()
                            .map(|f| f.name.split(" ").nth(1).unwrap().trim_end_matches(".sql").parse::<usize>().unwrap() )
                            .unwrap_or(0);
                        let new_file = OpenedFile {
                            path : None,
                            name : format!("Untitled {}.sql", n_untitled + 1),
                            saved : true,
                            content : None,
                            index : files.len(),
                            dt : Local::now().to_string()
                        };
                        files.push(new_file.clone());
                        on_new.call(new_file);
                    },
                    MultiArchiverAction::Add(file) => {
                        recent_files.push(file.clone());
                        on_added.call(file);
                    },
                    MultiArchiverAction::OpenRequest(path) => {
                        if files.len() == 16 {
                            send.send(MultiArchiverAction::OpenError(format!("File list limit reached"))).unwrap();
                            return glib::source::Continue(true);
                        }

                        println!("{:?}", files);
                        if files.iter().find(|f| f.path.as_ref().map(|p| &p[..] == &path[..] ).unwrap_or(false) ).is_some() {
                            send.send(MultiArchiverAction::OpenError(format!("File already opened"))).unwrap();
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

                        let handle = spawn_open_file(send.clone(), path, files.len());
                        file_open_handle = Some(handle);
                    },
                    MultiArchiverAction::CloseRequest(ix, force) => {

                        // This force=true branch will be hit by a request from the toast button
                        // clicked when the user wants to ignore an unsaved file. If win_close_request=true,
                        // the action originated from a application window close. If win_close_request=false,
                        // the action originated from a file list item close.
                        if force {
                            last_closed_file = Some(remove_file(&mut files, ix));
                            let n = files.len();
                            on_file_closed.call((ix, n));
                            println!("File closed");
                            if win_close_request {
                                on_window_close.call(());
                                win_close_request = false;
                            }
                        } else {
                            if files[ix].saved {
                                last_closed_file = Some(remove_file(&mut files, ix));
                                let n = files.len();
                                on_file_closed.call((ix, n));
                            } else {
                                on_close_confirm.call(files[ix].clone());
                            }
                        }
                        final_state.replace(recent_files.clone());
                    },
                    MultiArchiverAction::SaveRequest(opt_path) => {
                        if let Some(ix) = selected {
                            if let Some(path) = opt_path {
                                let content = on_buffer_read_request.call_with_values(ix).remove(0);
                                spawn_save_file(path, ix, content, send.clone());
                            } else {
                                if let Some(path) = files[ix].path.clone() {
                                    let content = on_buffer_read_request.call_with_values(ix).remove(0);
                                    spawn_save_file(path, ix, content, send.clone());
                                } else {
                                    on_save_unknown_path.call(files[ix].name.clone());
                                }
                            }
                        } else {
                            panic!("No file selected");
                        }
                    },
                    MultiArchiverAction::SaveSuccess(ix, path) => {
                        if files[ix].name.starts_with("Untitled") {
                            files[ix].name = path.clone();
                            files[ix].path = Some(path.clone());
                            on_name_changed.call((ix, path.clone()));

                            if recent_files.iter().find(|f| &f.path.as_ref().unwrap()[..] == &path[..] ).is_none() {
                                recent_files.push(files[ix].clone());
                            }
                        }
                        send.send(MultiArchiverAction::SetSaved(ix, true));
                    },
                    MultiArchiverAction::SaveError(e) => {
                        on_error.call(e);
                    },
                    MultiArchiverAction::SetSaved(ix, saved) => {

                        // SetSaved will be called when a buffer is cleared after a file is closed,
                        // so we just ignore the call in this case, since the file won't be at the
                        // buffer anymore (impl React<QueriesEditor> for MultiArchiver).
                        if last_closed_file.clone().map(|f| f.index == ix ).unwrap_or(false) {
                            last_closed_file = None;
                            return glib::source::Continue(true);
                        }

                        files[ix].saved = saved;
                        if saved {
                            on_file_persisted.call(files[ix].clone());
                        } else {
                            on_file_changed.call(files[ix].clone());
                        }
                    },
                    MultiArchiverAction::OpenSuccess(file) => {
                        files.push(file.clone());
                        println!("Files after opening = {:?}", files);
                        on_open.call(file.clone());
                        send.send(MultiArchiverAction::SetSaved(file.index, true));

                        if recent_files.iter().find(|f| &f.path.as_ref().unwrap()[..] == &file.path.as_ref().unwrap()[..] ).is_none() {
                            recent_files.push(file.clone());
                        }
                    },
                    MultiArchiverAction::OpenError(msg) => {
                        on_error.call(msg.clone());
                    },
                    MultiArchiverAction::Select(opt_ix) => {
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
                        final_state.replace(recent_files.clone());
                    },
                    // MultiArchiverAction::CloseConfirm(_) | MultiArchiverAction::Opened(_) | MultiArchiverAction::Closed(_) => {
                    //}
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
            on_save,
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
        match File::create(&path) {
            Ok(mut f) => {
                match f.write_all(content.as_bytes()) {
                    Ok(_) => {
                        send.send(MultiArchiverAction::SaveSuccess(index, path));
                        true
                    },
                    Err(e) => {
                        send.send(MultiArchiverAction::SaveError(format!("{}", e)));
                        false
                    }
                }
            },
            Err(e) => {
                send.send(MultiArchiverAction::SaveError(format!("{}", e)));
                false
            }
        }
    })
}

fn spawn_open_file(send : glib::Sender<MultiArchiverAction>, path : String, n_files : usize) -> JoinHandle<()> {
    thread::spawn(move || {
        match File::open(&path) {
            Ok(mut f) => {
                let mut content = String::new();
                if let Err(e) = f.read_to_string(&mut content) {
                    send.send(MultiArchiverAction::OpenError(format!("{}", e)));
                }

                if content.len() > MAX_FILE_SIZE {
                    send.send(MultiArchiverAction::OpenError(format!("File extrapolates maximum size"))).unwrap();
                    return;
                }

                let new_file = OpenedFile {
                    path : Some(path.clone()),
                    name : path.clone(),
                    saved : true,
                    content : Some(content),
                    index : n_files,
                    dt : Local::now().to_string()
                };
                send.send(MultiArchiverAction::OpenSuccess(new_file)).unwrap();
            },
            Err(e) => {
                send.send(MultiArchiverAction::OpenError(format!("{}", e ))).unwrap();
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
    pub dt : String,
    pub index : usize
}


