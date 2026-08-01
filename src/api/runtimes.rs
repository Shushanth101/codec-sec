use axum::{extract::State, Json};
use crate::models::RuntimeInfo;
use crate::AppState;

pub async fn get_runtimes(State(state): State<AppState>) -> Json<Vec<RuntimeInfo>> {
    let runtimes = state.registry.list();
    let runtime_infos: Vec<RuntimeInfo> = runtimes
        .into_iter()
        .map(|r| RuntimeInfo {
            id: r.id().to_string(),
            name: r.name().to_string(),
            compiled: r.compiled(),
        })
        .collect();

    Json(runtime_infos)
}
