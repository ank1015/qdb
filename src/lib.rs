//! qdb: a database engine for text files in nested folders, queried with
//! bash commands. This is the first stage: a simple, correct, in-memory
//! implementation with no indexes and no on-disk storage.
//!
//! ```
//! let mut db = qdb::Db::new();
//! qdb::execute(&mut db, b"mkdir -p src");
//! qdb::execute(&mut db, b"echo 'hello world' > src/a.txt");
//! let out = qdb::execute(&mut db, b"grep -rn hello .");
//! assert_eq!(out.stdout, b"./src/a.txt:1:hello world\n");
//! assert_eq!(out.code, 0);
//! ```

mod cmd;
pub mod engine;
pub mod glob;
pub mod regex;
pub mod shell;
pub mod store;
pub mod testing;
pub mod tree;
pub mod util;

pub use engine::{execute, execute_to, Outcome, Sink};
pub use tree::Db;
