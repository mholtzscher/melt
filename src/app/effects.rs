use std::path::PathBuf;

use crate::model::{ForgeType, GitInput};

use super::state::ListState;

#[derive(Debug, Clone)]
pub struct LockRequest {
    pub path: PathBuf,
    pub name: String,
    pub owner: String,
    pub repo: String,
    pub rev: String,
    pub forge_type: ForgeType,
    pub host: Option<String>,
}

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
        input: Box<GitInput>,
        parent_list: Box<ListState>,
    },
    Lock(LockRequest),
    CheckUpdates {
        inputs: Vec<GitInput>,
    },
}
