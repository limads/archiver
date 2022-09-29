/*
The multi and single archivers are stateful objects that keep track of opened
files matching what the user opens it its UI. Since The actual React<.> implementations
will be actually done by the dependent crates, they are expected to use those structures
via the newtype pattern. To reduce boilerplate, The newtype must only implement
the empty trait ArchiverImpl { } and also implement AsRef<Archiver> so that the default
method implementations can be used (the closest Rust have to inheritance). The specialization methods
can be accessed without anonymous zero-field access syntax (an idiomatic way to simulate inheritance,
since all application-specific archivers are actually expected to expose the full
archiver interface. The other options would be for the new type to be a SpecificArchiver(pub BaseArchiver)
(which would require all uses to access the base field, and would unneccessarily keep implementation
details public) or keep the field private and re-implement the methods, which is more error-prone).
*/

// TODO make sure paths to be saved, if they exist, never overwrite folders.

// TODO do nothing when the opened path is already the currently-opened file.

mod multi;

pub use multi::*;

mod single;

pub use single::*;

mod dialogs;

pub use dialogs::*;

mod actions;

pub use actions::*;

mod datadir;

pub use datadir::*;

mod config;

mod icons;

pub use icons::*;

pub use config::*;

pub fn log_err<E : std::error::Error>(err : E) {
    eprintln!("{}", err);
}


