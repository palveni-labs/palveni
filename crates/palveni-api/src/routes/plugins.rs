use axum::Json;
use axum::extract::State;
use palveni_plugin_host::{PluginSummary, RuntimeKind, skin_store::SkinStore};
use serde::Serialize;

use crate::auth::AdminUser;
use crate::{ApiError, AppState};

#[derive(utoipa::ToSchema, Serialize)]
pub struct PluginList {
    plugins: Vec<PluginSummary>,
}

pub async fn list(
    _: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<PluginList>, ApiError> {
    let mut plugins = state.plugins.list();
    let skins = super::skins::on_disk(state.skins, SkinStore::installed).await?;
    plugins.extend(skins.into_iter().map(|record| {
        let info = record.skin;
        PluginSummary {
            name: info
                .name
                .get("en")
                .or_else(|| info.name.get("zh-CN"))
                .cloned()
                .unwrap_or(info.id),
            id: info.plugin_id,
            version: info.version,
            runtime: RuntimeKind::None,
            enabled: true,
            provides: vec!["skin".to_owned()],
        }
    }));
    Ok(Json(PluginList { plugins }))
}
