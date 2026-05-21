//! Undo / redo stacks.
#![allow(dead_code, unused_variables, unused_imports)]

#[derive(Debug, Default)]
pub struct UndoRedo<T> { pub undo: Vec<T>, pub redo: Vec<T> }

