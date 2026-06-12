use std::path::PathBuf;

pub fn cinder_turn_workflow_path() -> PathBuf {
    workflow_path_for_id("cinder_turn")
}

pub fn workflow_path_for_id(workflow_id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("config")
        .join("workflows")
        .join(format!("{}.toml", workflow_id))
}

pub fn cinder_npc_tick_workflow_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("config")
        .join("workflows")
        .join("cinder_npc_tick.toml")
}

pub fn cinder_npc_turn_workflow_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("config")
        .join("workflows")
        .join("cinder_npc_turn.toml")
}
