//! The enabled routes and their public operation descriptions. The same
//! entries wire axum, populate the capability registry and describe OpenAPI.
use axum::routing::{MethodFilter, MethodRouter, on};
use palveni_core::features::{Capability, Permission, Risk};

use crate::{AppState, openapi, routes};

pub(super) struct Endpoint {
    pub capability: Capability,
    pub handler: fn() -> MethodRouter<AppState>,
    pub input: Input,
    pub output: Output,
    pub status: u16,
    pub errors: &'static [u16],
}

#[derive(Clone, Copy)]
pub(super) enum Input {
    None,
    Json(&'static str),
    Tar { required: bool },
}

#[derive(Clone, Copy)]
pub(super) enum Output {
    Json(&'static str),
    List(&'static str, &'static str),
    SkinManifest,
    Asset,
    Preview,
    Tar,
    Events,
    Document,
    Empty,
}

macro_rules! endpoint {
    ($id:ident, $module:ident, $method:ident $path:literal, $handler:path, $permission:ident, $risk:ident, $undo:literal,
     $description:literal, $input:expr, $status:literal $output:expr, [$($code:literal),*]) => {
        Endpoint {
            capability: Capability {
                id: stringify!($id), module: stringify!($module), method: stringify!($method),
                path: concat!("/api/v1", $path), permission: Permission::$permission,
                risk: Risk::$risk, reversible: $undo, description: $description,
            },
            handler: || on(MethodFilter::$method, $handler), input: $input,
            output: $output, status: $status, errors: &[$($code),*],
        }
    }
}

pub(super) fn endpoints() -> Vec<Endpoint> {
    use Input::{Json as Body, None};
    use Output::*;
    vec![
        endpoint!(getSystemInfo, system, GET "/system/info", routes::system::info, Public, Read, false,
            "Read the server identity, setup status and enabled modules.", None, 200 Json("SystemInfo"), []),
        endpoint!(getHealth, system, GET "/system/health", routes::system::health, Public, Read, false,
            "Check server liveness.", None, 200 Json("Health"), []),
        endpoint!(createAdministrator, auth, POST "/setup", routes::auth::setup, Public, Write, false,
            "Create the first administrator on an unconfigured server.", Body("SetupRequest"), 201 Json("SetupResponse"), [400,409]),
        endpoint!(login, auth, POST "/auth/login", routes::auth::login, Public, Write, true,
            "Authenticate a device and issue its access token.", Body("LoginRequest"), 200 Json("LoginResponse"), [400,401,429]),
        endpoint!(logout, auth, POST "/auth/logout", routes::auth::logout, Authenticated, Write, false,
            "Revoke the current device session.", None, 204 Empty, []),
        endpoint!(getCurrentUser, auth, GET "/auth/me", routes::auth::me, Authenticated, Read, false,
            "Read the authenticated user.", None, 200 Json("User"), []),
        endpoint!(listLibraries, libraries, GET "/libraries", routes::libraries::list, Authenticated, Read, false,
            "List media libraries.", None, 200 Json("LibraryList"), []),
        endpoint!(createLibrary, libraries, POST "/libraries", routes::libraries::create, Admin, Write, true,
            "Register a media library and its local paths.", Body("CreateLibraryRequest"), 201 Json("Library"), [400,409]),
        endpoint!(getLibrary, libraries, GET "/libraries/{id}", routes::libraries::get_one, Authenticated, Read, false,
            "Read a media library.", None, 200 Json("Library"), [404]),
        endpoint!(deleteLibrary, libraries, DELETE "/libraries/{id}", routes::libraries::delete, Admin, Destructive, false,
            "Delete a library and its index while preserving the media files.", None, 204 Empty, [404]),
        endpoint!(scanLibrary, libraries, POST "/libraries/{id}/scan", routes::libraries::scan, Admin, Write, false,
            "Queue a file scan or return the library's active scan.", None, 202 Json("ScanAccepted"), [404]),
        endpoint!(listItems, libraries, GET "/items", routes::items::list, Authenticated, Read, false,
            "Page through one library folder.", None, 200 Json("ItemList"), [400,404]),
        endpoint!(getItem, libraries, GET "/items/{id}", routes::items::get_one, Authenticated, Read, false,
            "Read an indexed media item.", None, 200 Json("Item"), [404]),
        endpoint!(listJobs, jobs, GET "/jobs", routes::jobs::list, Admin, Read, false,
            "List recent background jobs.", None, 200 Json("JobList"), [400]),
        endpoint!(getHardware, hardware, GET "/system/hardware", routes::system::hardware, Admin, Read, false,
            "Read the hardware report, optionally probing encoders again.", None, 200 Json("HardwareReport"), [400]),
        endpoint!(listPlugins, plugins, GET "/plugins", routes::plugins::list, Admin, Read, false,
            "List installed plugins.", None, 200 Json("PluginList"), []),
        endpoint!(listInstalledSkins, skins, GET "/skins/installed", routes::skins::installed, Authenticated, Read, false,
            "List installed skin packages.", None, 200 List("skins","InstalledSkin"), []),
        endpoint!(listSkinRegistry, skins, GET "/skins/registry", routes::skins::registry, Admin, Read, false,
            "List official skins pinned to this release.", None, 200 List("skins","RegistrySkin"), []),
        endpoint!(installOfficialSkin, skins, POST "/skins/registry/{id}/install", routes::skins::install, Admin, Write, true,
            "Install an official skin from its pinned package or the bundled copy.", Input::Tar {required:false}, 200 Json("InstalledSkin"), [400,404,409,413,503]),
        endpoint!(uploadSkin, skins, POST "/skins/upload", routes::skins::upload, Admin, Write, true,
            "Validate and install a third-party data-only skin archive.", Input::Tar {required:true}, 201 Json("InstalledSkin"), [400,409,413,503]),
        endpoint!(removeSkin, skins, DELETE "/skins/installed/{id}", routes::skins::remove, Admin, Destructive, false,
            "Uninstall a skin and its assets.", None, 204 Empty, [404]),
        endpoint!(getSkinManifest, skins, GET "/skins/installed/{id}/skin.json", routes::skins::manifest, Public, Read, false,
            "Read an installed skin manifest.", None, 200 SkinManifest, [404]),
        endpoint!(getSkinAsset, skins, GET "/skins/installed/{id}/assets/{path}", routes::skins::asset, Public, Read, false,
            "Read a validated font or bitmap by its package-relative path.", None, 200 Asset, [404]),
        endpoint!(getSkinPreview, skins, GET "/skins/registry/{id}/preview", routes::skins::preview, Public, Read, false,
            "Read the preview bitmap of an official skin.", None, 200 Preview, [404,503]),
        endpoint!(downloadSkinPackage, skins, GET "/skins/registry/{id}/package", routes::skins::package, Public, Read, false,
            "Download the verified bundled archive of an official skin.", None, 200 Tar, [404,503]),
        endpoint!(getRuntimeSettings, settings, GET "/system/settings", routes::settings::get, Admin, Read, false,
            "Read the active runtime setting overrides.", None, 200 Json("RuntimeSettings"), []),
        endpoint!(updateRuntimeSettings, settings, PUT "/system/settings", routes::settings::put, Admin, Write, true,
            "Replace server name and performance tier overrides.", Body("RuntimeSettings"), 200 Json("RuntimeSettings"), [400]),
        endpoint!(listCapabilities, system, GET "/system/capabilities", openapi::capabilities, Authenticated, Read, false,
            "List the operations available to this caller.", None, 200 List("capabilities","Capability"), []),
        endpoint!(getOpenApi, system, GET "/openapi.json", openapi::document, Public, Read, false,
            "Read the OpenAPI 3.1 description.", None, 200 Document, []),
        endpoint!(streamEvents, events, GET "/events", routes::events::stream, Authenticated, Read, false,
            "Stream visible changes and resume from the Last-Event-ID header.", None, 200 Events, [400]),
    ]
}
