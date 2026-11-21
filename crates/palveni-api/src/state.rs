use std::sync::Arc;

use palveni_core::{EventBus, FeatureRegistry, ServerId};
use palveni_db::Db;
use palveni_jobs::JobService;
use palveni_media::HardwareService;
use palveni_plugin_host::{PluginCatalog, skin_store::SkinStore};
use tokio_util::sync::CancellationToken;

/// Static facts reported by `GET /system/info`.
#[derive(Clone, Debug)]
pub struct ServerInfo {
    pub id: ServerId,
    pub name: String,
    pub version: String,
}

/// Shared application state. Cheap to clone.
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub music_search: Arc<palveni_library::music_search::MusicSearch>,
    pub identity: Arc<palveni_link::NodeIdentity>,
    pub login_limit: Arc<crate::LoginLimit>,
    pub subsonic_limit: Arc<crate::LoginLimit>,
    pub bus: EventBus,
    pub jobs: JobService,
    pub hardware: Arc<HardwareService>,
    pub plugins: Arc<PluginCatalog>,
    pub settings: Arc<tokio::sync::RwLock<palveni_core::settings::RuntimeSettings>>,
    pub boot_tier: palveni_core::PerfTier,
    pub skins: Arc<SkinStore>,
    pub skin_writes: Arc<tokio::sync::Semaphore>,
    pub features: FeatureRegistry,
    pub server: Arc<ServerInfo>,
    /// Cancelled on shutdown; ends long-lived responses such as SSE so
    /// graceful shutdown can complete.
    pub shutdown: CancellationToken,
}
