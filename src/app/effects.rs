use std::path::PathBuf;

use crate::model::{FlakeInput, GitInput};

use super::state::ListState;

#[derive(Debug, Clone)]
pub enum Effect {
    LoadFlake,
    Update {
        path: PathBuf,
        names: Vec<String>,
    },
    UpdateAll {
        path: PathBuf,
    },
    LoadChangelog {
        input: GitInput,
        parent_list: ListState,
    },
    Lock {
        path: PathBuf,
        name: String,
        lock_url: String,
    },
    CheckUpdates {
        inputs: Vec<FlakeInput>,
    },
}
