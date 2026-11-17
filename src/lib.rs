//! qdb: a database engine for text files in nested folders, queried with
//! bash commands. This is the first stage: a simple, correct, in-memory
//! implementation with no indexes and no on-disk storage.

pub mod glob;
pub mod regex;
pub mod shell;
pub mod tree;
