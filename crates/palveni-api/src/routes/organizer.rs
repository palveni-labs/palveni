//! Reversible Media Organizer: Dry-Run Template Preview, Safe Apply & 1-Click Undo
//! (`module: organizer`).

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path as StdPath, PathBuf};

use axum::Json;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use palveni_core::ServerEvent;
use palveni_core::assistant::{Block, ChangeColumns, ChangeGroup, ChangeRow};
use palveni_core::downloads::{
    OrganizeApplyRequest, OrganizeBatchList, OrganizeBatchSummary, OrganizeCandidate, OrganizeMode,
    OrganizeOperationRecord, OrganizePreviewRequest, OrganizePreviewResponse,
    builtin_organize_presets,
};
use palveni_core::ids::ItemId;
use palveni_db::audit::{AuditEntry, actions};
use palveni_db::downloads::OrganizeItemMeta;
use serde::Deserialize;
use serde_json::json;

use crate::auth::{AdminUser, CurrentUser};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, audit, parse_limit};
use crate::state::AppState;

const DEFAULT_PREVIEW_LIMIT: u32 = 200;

#[derive(Deserialize)]
pub struct BatchesQuery {
    limit: Option<String>,
}

/// Validates a relative template or subdirectory string, rejecting path traversal (`..`),
/// absolute prefixes, backslashes-as-escapes, and control characters.
pub(crate) fn validate_relative_template(
    raw: &str,
    field: &'static str,
) -> Result<String, ApiError> {
    let trimmed = raw.trim().trim_matches('/');
    if trimmed.is_empty() || trimmed.chars().count() > 255 || trimmed.chars().any(char::is_control)
    {
        return Err(
            ApiError::validation("Template must contain 1 to 255 printable characters")
                .field(field),
        );
    }
    if trimmed.contains('\\') {
        return Err(
            ApiError::validation("Use forward slashes '/' for directory separators").field(field),
        );
    }
    for segment in trimmed.split('/') {
        let seg = segment.trim();
        if seg.is_empty() || seg == "." || seg == ".." {
            return Err(ApiError::validation(
                "Relative path must not contain '..' or empty segments",
            )
            .field(field));
        }
    }
    Ok(trimmed.to_owned())
}

fn sanitize_segment_value(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    let clean = collapsed.trim_matches(['.', ' ']);
    if clean.is_empty() {
        "Untitled".to_owned()
    } else {
        clean.to_owned()
    }
}

fn render_item_relative_path(
    template: &str,
    target_subdir: Option<&str>,
    meta: &OrganizeItemMeta,
) -> Result<PathBuf, ApiError> {
    let ext = StdPath::new(&meta.path)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty());

    let title = sanitize_segment_value(&meta.title);
    let year_str = meta.year.map(|y| y.to_string()).unwrap_or_default();
    let season_str = format!("{:02}", meta.season.unwrap_or(1).max(0));
    let episode_str = format!("{:02}", meta.episode.unwrap_or(1).max(0));
    let artist_str = meta
        .artist
        .as_deref()
        .map(sanitize_segment_value)
        .unwrap_or_else(|| "未知艺人".to_owned());
    let author_str = meta
        .author
        .as_deref()
        .map(sanitize_segment_value)
        .unwrap_or_else(|| "未知作者".to_owned());

    let mut rendered_segments = Vec::new();
    if let Some(sub) = target_subdir {
        for seg in sub.split('/') {
            let clean = sanitize_segment_value(seg);
            if clean == "." || clean == ".." {
                return Err(ApiError::conflict("Path traversal segment rejected"));
            }
            rendered_segments.push(clean);
        }
    }

    let raw_parts: Vec<&str> = template.split('/').collect();
    for (idx, part) in raw_parts.iter().enumerate() {
        let mut expanded = (*part)
            .replace("{title}", &title)
            .replace("{year}", &year_str)
            .replace("{season}", &season_str)
            .replace("{episode}", &episode_str)
            .replace("{artist}", &artist_str)
            .replace("{author}", &author_str);
        // Clean up empty year parens like "Title ()" or "Title []"
        expanded = expanded
            .replace("()", "")
            .replace("[]", "")
            .replace("（）", "");
        let mut seg = sanitize_segment_value(&expanded);
        if idx + 1 == raw_parts.len()
            && let Some(ext_str) = ext
        {
            let suffix = format!(".{ext_str}");
            if !seg
                .to_ascii_lowercase()
                .ends_with(&suffix.to_ascii_lowercase())
            {
                seg.push('.');
                seg.push_str(ext_str);
            }
        }
        if seg == "." || seg == ".." {
            return Err(ApiError::conflict("Invalid target filename"));
        }
        rendered_segments.push(seg);
    }

    let mut rel = PathBuf::new();
    for seg in rendered_segments {
        rel.push(seg);
    }
    for comp in rel.components() {
        if !matches!(comp, Component::Normal(_)) {
            return Err(ApiError::conflict(
                "Target path must remain strictly relative to library root",
            ));
        }
    }
    Ok(rel)
}

fn find_owning_root<'a>(roots: &'a [String], item_path: &str) -> Option<(&'a str, PathBuf)> {
    let path = StdPath::new(item_path);
    let mut best: Option<(&'a str, PathBuf)> = None;
    for root in roots {
        if let Ok(rel) = path.strip_prefix(root)
            && best
                .as_ref()
                .is_none_or(|(prev, _)| root.len() > prev.len())
        {
            best = Some((root.as_str(), rel.to_path_buf()));
        }
    }
    best
}

fn detect_read_only_root(roots: &[String]) -> bool {
    let ro_mounts = proc_ro_mount_points();
    for root in roots {
        let p = StdPath::new(root);
        if let Ok(meta) = std::fs::metadata(p)
            && meta.permissions().readonly()
        {
            return true;
        }
        if ro_mounts
            .iter()
            .any(|mp| if mp == "/" { false } else { p.starts_with(mp) })
        {
            return true;
        }
    }
    false
}

fn proc_ro_mount_points() -> Vec<PathBuf> {
    let Ok(text) = std::fs::read_to_string("/proc/mounts") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let _dev = parts.next();
        let Some(mount_point) = parts.next() else {
            continue;
        };
        let _fs = parts.next();
        let Some(opts) = parts.next() else { continue };
        if opts.split(',').any(|o| o == "ro") && mount_point != "/" {
            out.push(PathBuf::from(mount_point));
        }
    }
    out
}

/// Ensures `path` is strictly inside `root`, contains no `..`, and traverses zero symlinks.
fn ensure_safe_path_inside_root(
    root: &StdPath,
    target: &StdPath,
    check_leaf_symlink: bool,
) -> Result<(), ApiError> {
    let root_meta = std::fs::symlink_metadata(root)
        .map_err(|_| ApiError::conflict("Library root directory is not accessible"))?;
    if root_meta.file_type().is_symlink() {
        return Err(ApiError::conflict(
            "Library root must not be a symbolic link",
        ));
    }
    if root_meta.permissions().readonly() {
        return Err(ApiError::conflict("Library root is mounted read-only"));
    }

    let rel = target
        .strip_prefix(root)
        .map_err(|_| ApiError::conflict("Target path escapes library root directory"))?;
    if rel.as_os_str().is_empty() {
        return Err(ApiError::conflict(
            "Target path cannot be library root itself",
        ));
    }

    let mut cursor = root.to_path_buf();
    let comps: Vec<Component<'_>> = rel.components().collect();
    for (idx, comp) in comps.iter().enumerate() {
        match comp {
            Component::Normal(part) => cursor.push(part),
            _ => {
                return Err(ApiError::conflict(
                    "Path contains disallowed traversal component",
                ));
            }
        }
        let is_leaf = idx + 1 == comps.len();
        if (!is_leaf || check_leaf_symlink)
            && let Ok(meta) = std::fs::symlink_metadata(&cursor)
        {
            if meta.file_type().is_symlink() {
                return Err(ApiError::conflict(
                    "Symbolic links inside organized paths are rejected",
                ));
            }
            if !is_leaf && meta.permissions().readonly() {
                return Err(ApiError::conflict("Target directory is read-only"));
            }
        }
    }

    let canonical_root = std::fs::canonicalize(root)
        .map_err(|_| ApiError::conflict("Cannot resolve library root path"))?;
    // Walk up to the deepest existing ancestor of `target` and verify canonical containment.
    let mut existing_ancestor = target.to_path_buf();
    while !existing_ancestor.exists() {
        if !existing_ancestor.pop() {
            break;
        }
    }
    if existing_ancestor.exists() {
        let canonical_anc = std::fs::canonicalize(&existing_ancestor)
            .map_err(|_| ApiError::conflict("Cannot resolve target ancestor path"))?;
        if !canonical_anc.starts_with(&canonical_root) {
            return Err(ApiError::conflict(
                "Resolved path escapes canonical library root",
            ));
        }
    }

    Ok(())
}

fn map_fs_write_err(err: std::io::Error, context: &str) -> ApiError {
    if matches!(
        err.kind(),
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::ReadOnlyFilesystem
    ) || err.raw_os_error() == Some(30)
    {
        return ApiError::conflict(format!("Read-only or permission denied ({context}): {err}"));
    }
    if err.kind() == std::io::ErrorKind::AlreadyExists {
        return ApiError::conflict(format!("Destination file already exists ({context})"));
    }
    ApiError::conflict(format!("Filesystem organize error ({context}): {err}"))
}

pub(crate) async fn build_preview_internal(
    state: &AppState,
    req: &OrganizePreviewRequest,
) -> Result<OrganizePreviewResponse, ApiError> {
    let template = validate_relative_template(&req.template, "template")?;
    let target_subdir = req
        .target_subdir
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| validate_relative_template(s, "targetSubdir"))
        .transpose()?;

    let library = state
        .db
        .libraries()
        .get(req.library_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Library not found"))?;

    let limit = u64::from(req.limit.unwrap_or(DEFAULT_PREVIEW_LIMIT).clamp(1, 500));
    let metas = state
        .db
        .downloads()
        .library_organize_items(library.id, limit)
        .await?;

    let read_only_mount = detect_read_only_root(&library.paths);

    // First pass: compute target paths & count target path occurrences to detect batch collisions.
    let mut target_counts: HashMap<String, usize> = HashMap::new();
    let mut staged = Vec::with_capacity(metas.len());

    for (idx, meta) in metas.iter().enumerate() {
        let Some((root, rel_before_path)) = find_owning_root(&library.paths, &meta.path) else {
            continue;
        };
        let rel_after_path = render_item_relative_path(&template, target_subdir.as_deref(), meta)?;
        let target_full = StdPath::new(root).join(&rel_after_path);
        let target_str = target_full.to_string_lossy().into_owned();
        *target_counts.entry(target_str.clone()).or_insert(0) += 1;
        staged.push((
            idx,
            meta,
            root,
            rel_before_path.to_string_lossy().into_owned(),
            rel_after_path.to_string_lossy().into_owned(),
            target_str,
        ));
    }

    let mut candidates = Vec::with_capacity(staged.len());
    let mut change_rows = Vec::with_capacity(staged.len());
    let mut collision_count = 0usize;
    let mut ready_count = 0usize;

    for (idx, meta, root, rel_before, rel_after, target_path) in staged {
        let row_id = format!("org-{idx}");
        let is_symlink = std::fs::symlink_metadata(&meta.path)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        let same_path = target_path == meta.path;
        let dup_target = target_counts.get(&target_path).copied().unwrap_or(0) > 1;
        let disk_exists = !same_path && StdPath::new(&target_path).exists();
        let root_ro = read_only_mount
            || std::fs::metadata(root)
                .map(|m| m.permissions().readonly())
                .unwrap_or(false);

        let (conflict, note, selectable, group) = if is_symlink {
            (
                Some("symlink".to_owned()),
                Some("符号链接已跳过（安全策略禁止）".to_owned()),
                false,
                "blocked",
            )
        } else if same_path {
            (
                Some("unchanged".to_owned()),
                Some("文件名已符合模板，无需改动".to_owned()),
                false,
                "unchanged",
            )
        } else if dup_target || disk_exists {
            collision_count = collision_count.saturating_add(1);
            (
                Some("collision".to_owned()),
                Some("目标路径存在同名冲突，已自动取消勾选".to_owned()),
                false,
                "conflict",
            )
        } else if root_ro {
            (
                Some("read_only".to_owned()),
                Some("所在挂载目录为只读，无法写入".to_owned()),
                false,
                "blocked",
            )
        } else {
            ready_count = ready_count.saturating_add(1);
            (None, None, true, "ready")
        };

        change_rows.push(ChangeRow {
            id: row_id.clone(),
            before: rel_before.clone(),
            after: rel_after.clone(),
            detail: Some(match req.mode {
                OrganizeMode::Rename => "原地重命名并同步索引".to_owned(),
                OrganizeMode::Hardlink => "创建硬链接并更新索引".to_owned(),
            }),
            selected: selectable,
            group: Some(group.to_owned()),
            size_bytes: meta.size_bytes,
            thumbs: None,
            relation: None,
            note,
        });

        candidates.push(OrganizeCandidate {
            row_id,
            item_id: meta.item_id,
            source_path: meta.path.clone(),
            target_path,
            relative_before: rel_before,
            relative_after: rel_after,
            size_bytes: meta.size_bytes,
            conflict,
            selectable,
        });
    }

    let summary_text = if read_only_mount {
        format!(
            "注意：「{}」所在挂载为只读，仅支持预览，写入将被拦截。",
            library.name
        )
    } else if collision_count > 0 {
        format!("共 {ready_count} 项可安全整理，另有 {collision_count} 项存在路径冲突已跳过。")
    } else {
        format!("共 {ready_count} 项可安全整理，执行后支持一键撤销恢复原路径。")
    };

    let preview_block = Block::ChangePreview {
        id: "organizer-preview".into(),
        title: Some(format!("「{}」整理变更预览", library.name)),
        summary: Some(summary_text),
        rows: change_rows,
        groups: Some(vec![
            ChangeGroup {
                id: "ready".into(),
                label: "可执行变更".into(),
                collapse_after: Some(25),
            },
            ChangeGroup {
                id: "conflict".into(),
                label: "路径冲突".into(),
                collapse_after: Some(10),
            },
            ChangeGroup {
                id: "blocked".into(),
                label: "只读或受限".into(),
                collapse_after: Some(10),
            },
            ChangeGroup {
                id: "unchanged".into(),
                label: "无需改动".into(),
                collapse_after: Some(5),
            },
        ]),
        columns: Some(ChangeColumns {
            item: Some("原相对路径".into()),
            change: Some("整理后路径".into()),
            reason: Some("方式与检查".into()),
        }),
    };

    Ok(OrganizePreviewResponse {
        library_id: library.id,
        library_name: library.name,
        mode: req.mode,
        template,
        read_only_mount,
        collision_count,
        ready_count,
        candidates,
        preview_block,
        presets: builtin_organize_presets(),
    })
}

pub(crate) async fn apply_organize_internal(
    state: &AppState,
    actor: &CurrentUser,
    req: &OrganizeApplyRequest,
) -> Result<OrganizeBatchSummary, ApiError> {
    if !actor.is_admin() {
        return Err(ApiError::forbidden());
    }
    let template = validate_relative_template(&req.template, "template")?;
    let target_subdir = req
        .target_subdir
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| validate_relative_template(s, "targetSubdir"))
        .transpose()?;

    let library = state
        .db
        .libraries()
        .get(req.library_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Library not found"))?;

    if detect_read_only_root(&library.paths) {
        return Err(ApiError::conflict(
            "Library storage mount is read-only; cannot apply file organization",
        ));
    }

    let metas = state
        .db
        .downloads()
        .library_organize_items(library.id, 500)
        .await?;

    let filter_ids: HashSet<ItemId> = req.selected_item_ids.iter().copied().collect();
    let chosen_metas: Vec<&OrganizeItemMeta> = metas
        .iter()
        .filter(|m| filter_ids.is_empty() || filter_ids.contains(&m.item_id))
        .collect();

    if chosen_metas.is_empty() {
        return Err(
            ApiError::validation("No matching library items selected").field("selectedItemIds")
        );
    }

    let mut planned: Vec<(ItemId, PathBuf, PathBuf, PathBuf)> = Vec::new();
    let mut seen_targets: HashSet<PathBuf> = HashSet::new();

    for meta in chosen_metas {
        let Some((root_str, _)) = find_owning_root(&library.paths, &meta.path) else {
            return Err(ApiError::conflict(
                "Item path is outside configured library roots",
            ));
        };
        let root_path = PathBuf::from(root_str);
        let src_path = PathBuf::from(&meta.path);

        ensure_safe_path_inside_root(&root_path, &src_path, true)?;
        let src_meta = std::fs::symlink_metadata(&src_path)
            .map_err(|_| ApiError::conflict(format!("Source file missing: {}", meta.path)))?;
        if !src_meta.file_type().is_file() {
            return Err(ApiError::conflict(
                "Only regular media files can be organized",
            ));
        }

        let rel_target = render_item_relative_path(&template, target_subdir.as_deref(), meta)?;
        let dst_path = root_path.join(rel_target);
        if dst_path == src_path {
            continue;
        }
        ensure_safe_path_inside_root(&root_path, &dst_path, true)?;

        if dst_path.exists() {
            return Err(ApiError::conflict(format!(
                "Target path already exists: {}",
                dst_path.display()
            )));
        }
        if !seen_targets.insert(dst_path.clone()) {
            return Err(ApiError::conflict(format!(
                "Multiple items collide on target path: {}",
                dst_path.display()
            )));
        }
        planned.push((meta.item_id, root_path, src_path, dst_path));
    }

    if planned.is_empty() {
        return Err(ApiError::validation(
            "Selected items already match target naming template",
        ));
    }

    // Execute filesystem operations with rollback on partial failure!
    let mut completed_ops: Vec<OrganizeOperationRecord> = Vec::with_capacity(planned.len());
    for (item_id, root_path, src_path, dst_path) in planned {
        let mut created_dirs = Vec::new();
        if let Some(parent) = dst_path.parent() {
            let mut missing = Vec::new();
            let mut cur = parent.to_path_buf();
            while cur != root_path && !cur.exists() {
                missing.push(cur.clone());
                if !cur.pop() {
                    break;
                }
            }
            missing.reverse();
            for dir in missing {
                if let Err(err) = std::fs::create_dir(&dir) {
                    rollback_fs_operations(req.mode, &completed_ops);
                    return Err(map_fs_write_err(err, "create_dir"));
                }
                created_dirs.push(dir.to_string_lossy().into_owned());
            }
        }

        let fs_res = match req.mode {
            OrganizeMode::Rename => std::fs::rename(&src_path, &dst_path),
            OrganizeMode::Hardlink => std::fs::hard_link(&src_path, &dst_path),
        };
        if let Err(err) = fs_res {
            for d in created_dirs.iter().rev() {
                let _ = std::fs::remove_dir(d);
            }
            rollback_fs_operations(req.mode, &completed_ops);
            return Err(map_fs_write_err(err, req.mode.as_str()));
        }

        completed_ops.push(OrganizeOperationRecord {
            item_id,
            from_path: src_path.to_string_lossy().into_owned(),
            to_path: dst_path.to_string_lossy().into_owned(),
            created_dirs,
        });
    }

    let batch = match state
        .db
        .downloads()
        .record_organize_batch(
            actor.user.id,
            library.id,
            req.mode,
            &template,
            &completed_ops,
        )
        .await
    {
        Ok(b) => b,
        Err(db_err) => {
            rollback_fs_operations(req.mode, &completed_ops);
            return Err(db_err.into());
        }
    };

    if let Some(updated_lib) = state.db.libraries().get(library.id).await? {
        state.bus.publish(ServerEvent::LibraryUpdated(updated_lib));
    }

    audit(
        state,
        AuditEntry {
            action: actions::ORGANIZER_APPLIED,
            actor: Some(actor.user.id),
            target: Some(batch.id.clone()),
            detail: Some(json!({
                "libraryId": library.id,
                "mode": req.mode.as_str(),
                "template": template,
                "count": batch.operation_count,
            })),
        },
    )
    .await;

    Ok(batch)
}

fn rollback_fs_operations(mode: OrganizeMode, ops: &[OrganizeOperationRecord]) {
    for op in ops.iter().rev() {
        match mode {
            OrganizeMode::Rename => {
                let _ = std::fs::rename(&op.to_path, &op.from_path);
            }
            OrganizeMode::Hardlink => {
                if StdPath::new(&op.from_path).exists() {
                    let _ = std::fs::remove_file(&op.to_path);
                } else {
                    let _ = std::fs::rename(&op.to_path, &op.from_path);
                }
            }
        }
        for dir in op.created_dirs.iter().rev() {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

/// `POST /api/v1/organizer/preview`
pub async fn preview(
    AdminUser(_admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<OrganizePreviewRequest>, ApiError>,
) -> Result<Json<OrganizePreviewResponse>, ApiError> {
    let ApiJson(req) = body?;
    let resp = build_preview_internal(&state, &req).await?;
    Ok(Json(resp))
}

/// `POST /api/v1/organizer/apply`
pub async fn apply(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<OrganizeApplyRequest>, ApiError>,
) -> Result<Json<OrganizeBatchSummary>, ApiError> {
    let ApiJson(req) = body?;
    let summary = apply_organize_internal(&state, &admin, &req).await?;
    Ok(Json(summary))
}

/// `GET /api/v1/organizer/batches`
pub async fn list_batches(
    AdminUser(_admin): AdminUser,
    State(state): State<AppState>,
    query: Result<Query<BatchesQuery>, QueryRejection>,
) -> Result<Json<OrganizeBatchList>, ApiError> {
    let Query(q) = query?;
    let limit = parse_limit(q.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    let batches = state
        .db
        .downloads()
        .list_organize_batches(u64::from(limit))
        .await?;
    Ok(Json(OrganizeBatchList {
        batches,
        presets: builtin_organize_presets(),
    }))
}

/// `POST /api/v1/organizer/batches/{id}/undo`
pub async fn undo_batch(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<OrganizeBatchSummary>, ApiError> {
    let id = path?.0;
    let batch = state
        .db
        .downloads()
        .get_organize_batch(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("Organize batch not found"))?;
    if batch.undone_at.is_some() {
        return Err(ApiError::conflict("Organize batch has already been undone"));
    }

    let library = state
        .db
        .libraries()
        .get(batch.library_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Library not found"))?;

    if detect_read_only_root(&library.paths) {
        return Err(ApiError::conflict(
            "Library storage mount is read-only; cannot undo batch",
        ));
    }

    // Validate all paths remain inside library roots before touching disk.
    for op in &batch.operations {
        let Some((root_str, _)) = find_owning_root(&library.paths, &op.from_path) else {
            return Err(ApiError::conflict("Original path outside library roots"));
        };
        let root_path = PathBuf::from(root_str);
        ensure_safe_path_inside_root(&root_path, StdPath::new(&op.from_path), true)?;
        ensure_safe_path_inside_root(&root_path, StdPath::new(&op.to_path), true)?;
    }

    // Reverse file operations in reverse order.
    for (reversed_count, op) in batch.operations.iter().rev().enumerate() {
        if let Some(orig_parent) = StdPath::new(&op.from_path).parent() {
            let _ = std::fs::create_dir_all(orig_parent);
        }
        let res = match batch.mode {
            OrganizeMode::Rename => std::fs::rename(&op.to_path, &op.from_path),
            OrganizeMode::Hardlink => {
                if StdPath::new(&op.from_path).exists() {
                    std::fs::remove_file(&op.to_path)
                } else {
                    std::fs::rename(&op.to_path, &op.from_path)
                }
            }
        };
        if let Err(err) = res {
            // Roll forward any ops we already undid in this attempt
            for prev in batch
                .operations
                .iter()
                .rev()
                .take(reversed_count)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                match batch.mode {
                    OrganizeMode::Rename => {
                        let _ = std::fs::rename(&prev.from_path, &prev.to_path);
                    }
                    OrganizeMode::Hardlink => {
                        let _ = std::fs::hard_link(&prev.from_path, &prev.to_path);
                    }
                }
            }
            return Err(map_fs_write_err(err, "undo"));
        }
        for dir in op.created_dirs.iter().rev() {
            let _ = std::fs::remove_dir(dir);
        }
    }

    let updated = state
        .db
        .downloads()
        .mark_organize_batch_undone(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("Organize batch not found"))?;

    if let Some(updated_lib) = state.db.libraries().get(library.id).await? {
        state.bus.publish(ServerEvent::LibraryUpdated(updated_lib));
    }

    audit(
        &state,
        AuditEntry {
            action: actions::ORGANIZER_UNDONE,
            actor: Some(admin.user.id),
            target: Some(updated.id.clone()),
            detail: Some(json!({
                "libraryId": updated.library_id,
                "mode": updated.mode.as_str(),
                "count": updated.operation_count,
            })),
        },
    )
    .await;

    Ok(Json(updated))
}
