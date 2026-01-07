//! UI rendering functions
//!
//! This module contains all the rendering logic for the application,
//! separated by view type.

mod changelog;
mod common;
mod list;

pub use changelog::render_changelog;
pub use common::{render_error, render_loading};
pub use list::render_list;
