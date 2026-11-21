use axum::{
    Json,
    extract::{Path, State, rejection::PathRejection},
    http::{StatusCode, header},
};
use palveni_core::DeviceId;
use palveni_db::audit::AuditEntry;
use palveni_link::NodeId;
use serde::{Deserialize, Serialize};

use super::{audit, path_id};
use crate::{ApiError, AppState, auth::CurrentUser, json::ApiJson};

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinkChallenge {
    challenge: String,
    server_node_id: String,
    device_id: DeviceId,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinkPairRequest {
    node_id: String,
    signature: String,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinkPairing {
    device_id: DeviceId,
    server_node_id: String,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LinkDevice {
    id: DeviceId,
    name: String,
    node_id: String,
    connected: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct LinkDeviceList {
    devices: Vec<LinkDevice>,
}

pub async fn challenge(
    caller: CurrentUser,
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    state.link.peers()?;
    let device_id = caller.device_id.ok_or_else(ApiError::unauthorized)?;
    let server = state.identity.node_id();
    let challenge = state.link.challenge(server, device_id, caller.session_id)?;
    Ok((
        [(header::CACHE_CONTROL, "private, no-store")],
        Json(LinkChallenge {
            challenge,
            device_id,
            server_node_id: server.to_string(),
        }),
    ))
}

pub async fn pair(
    caller: CurrentUser,
    State(state): State<AppState>,
    ApiJson(request): ApiJson<LinkPairRequest>,
) -> Result<Json<LinkPairing>, ApiError> {
    let device_id = caller.device_id.ok_or_else(ApiError::unauthorized)?;
    let node: NodeId = request
        .node_id
        .parse()
        .map_err(|_| ApiError::validation("Invalid device node ID").field("nodeId"))?;
    let server = state.identity.node_id();
    if node == server {
        return Err(ApiError::validation("Cannot pair the server's own node ID").field("nodeId"));
    }
    let mut signature = [0u8; 64];
    hex::decode_to_slice(&request.signature, &mut signature)
        .map_err(|_| ApiError::validation("Invalid signature").field("signature"))?;
    state.link.verify(caller.session_id, node, &signature)?;
    // A disconnected HTTP caller must not cancel a commit between the two updates.
    tokio::spawn(async move {
        let device_id = {
            let _guard = state.link.changes.lock().await;
            let peers = state.link.peers()?;
            if state
                .db
                .sessions()
                .active_by_id(caller.session_id)
                .await?
                .is_none()
            {
                return Err(ApiError::unauthorized());
            }
            let paired = state
                .db
                .sessions()
                .pair_device(
                    caller.user.id,
                    device_id,
                    caller.session_id,
                    &node.to_string(),
                )
                .await?;
            peers.allow_incoming(node);
            paired
        };
        audit(
            &state,
            AuditEntry {
                action: "link.paired",
                actor: Some(caller.user.id),
                target: Some(device_id.to_string()),
                detail: None,
            },
        )
        .await;
        Ok(Json(LinkPairing {
            device_id,
            server_node_id: server.to_string(),
        }))
    })
    .await
    .map_err(ApiError::internal)?
}

pub async fn devices(
    caller: CurrentUser,
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    let peers = state.link.peers()?;
    let rows = state
        .db
        .sessions()
        .paired_devices(Some(caller.user.id))
        .await?;
    let devices = rows
        .into_iter()
        .map(|device| {
            let node = device.public_key.parse().map_err(ApiError::internal)?;
            Ok(LinkDevice {
                id: device.id,
                name: device.name,
                node_id: device.public_key,
                connected: peers.connected(node),
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Ok((
        [(header::CACHE_CONTROL, "private, no-store")],
        Json(LinkDeviceList { devices }),
    ))
}

pub async fn revoke(
    caller: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id: DeviceId = path_id(&path?.0, "Device")?;
    tokio::spawn(async move {
        {
            let _guard = state.link.changes.lock().await;
            let peers = state.link.peers()?;
            if state
                .db
                .sessions()
                .active_by_id(caller.session_id)
                .await?
                .is_none()
            {
                return Err(ApiError::unauthorized());
            }
            let key = state
                .db
                .sessions()
                .paired_device_key(caller.user.id, id)
                .await?
                .ok_or_else(|| ApiError::not_found("Paired device not found"))?;
            let node = key.parse().map_err(ApiError::internal)?;
            // Deny first: even a lost commit acknowledgment cannot leave the key admitted.
            peers.disconnect(node);
            if let Err(err) = state.db.sessions().revoke_device(id).await {
                if state
                    .db
                    .sessions()
                    .paired_device_key(caller.user.id, id)
                    .await?
                    .as_deref()
                    == Some(key.as_str())
                {
                    peers.allow_incoming(node);
                }
                return Err(err.into());
            }
        }
        audit(
            &state,
            AuditEntry {
                action: "link.revoked",
                actor: Some(caller.user.id),
                target: Some(id.to_string()),
                detail: None,
            },
        )
        .await;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
    .map_err(ApiError::internal)?
}
