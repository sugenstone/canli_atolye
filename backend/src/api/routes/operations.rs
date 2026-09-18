//! Faz 5 operasyon endpoint'leri: bloke (nedenli), notlar, dosya ekleri.

use crate::api::error::ApiResult;
use crate::api::extractor::AuthUser;
use crate::api::state::AppState;
use crate::application::services::operation_service as service;
use crate::domain::entities::{Attachment, BlockReason, ProcessBlock, ProcessExecution};
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

// --- Bloke kataloğu ---

/// GET /api/v1/block-reasons
pub async fn list_reasons(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<BlockReason>>> {
    Ok(Json(service::list_reasons(&state.pool, &auth.user).await?))
}

/// POST /api/v1/block-reasons (ADMIN)
#[derive(Debug, Deserialize)]
pub struct CreateReasonRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub async fn create_reason(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateReasonRequest>,
) -> ApiResult<(StatusCode, Json<BlockReason>)> {
    let created =
        service::create_reason(&state.pool, &auth.user, &req.name, req.description.as_deref())
            .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

// --- Bloke / çöz (nedenli) ---

#[derive(Debug, Deserialize)]
pub struct BlockRequest {
    pub reason_id: Uuid,
    #[serde(default)]
    pub description: Option<String>,
}

/// POST /api/v1/process-executions/{id}/block — reason zorunlu (MASTER PLAN §67)
pub async fn block(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(execution_id): Path<Uuid>,
    Json(req): Json<BlockRequest>,
) -> ApiResult<Json<ProcessExecution>> {
    let updated =
        service::block(&state.pool, &auth.user, execution_id, req.reason_id, req.description)
            .await?;
    crate::api::routes::insights::publish_execution_event(
        &state, updated.work_item_id, Some(updated.id), "BLOCKED",
    )
    .await;
    Ok(Json(updated))
}

#[derive(Debug, Deserialize, Default)]
pub struct UnblockRequest {
    #[serde(default)]
    pub resolution_note: Option<String>,
}

/// POST /api/v1/process-executions/{id}/unblock (ADMIN/PM)
pub async fn unblock(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(execution_id): Path<Uuid>,
    body: Option<Json<UnblockRequest>>,
) -> ApiResult<Json<ProcessExecution>> {
    let note = body.and_then(|Json(req)| req.resolution_note);
    let updated = service::unblock(&state.pool, &auth.user, execution_id, note).await?;
    crate::api::routes::insights::publish_execution_event(
        &state, updated.work_item_id, Some(updated.id), "UNBLOCKED",
    )
    .await;
    Ok(Json(updated))
}

/// GET /api/v1/process-executions/{id}/blocks
pub async fn list_blocks(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(execution_id): Path<Uuid>,
) -> ApiResult<Json<Vec<ProcessBlock>>> {
    Ok(Json(service::blocks_of_execution(&state.pool, &auth.user, execution_id).await?))
}

// --- Notlar ---

#[derive(Debug, Deserialize)]
pub struct NoteRequest {
    pub note: String,
}

/// POST /api/v1/process-executions/{id}/notes
pub async fn add_note(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(execution_id): Path<Uuid>,
    Json(req): Json<NoteRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    service::add_note(&state.pool, &auth.user, execution_id, req.note).await?;
    crate::api::routes::insights::publish_for_execution(&state, execution_id, "NOTE_ADDED").await;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// --- Dosya ekleri ---

/// POST /api/v1/attachments (multipart: file, entity_type, entity_id)
pub async fn upload(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<Attachment>)> {
    let mut entity_type: Option<String> = None;
    let mut entity_id: Option<Uuid> = None;
    let mut file_name: Option<String> = None;
    let mut mime_type: Option<String> = None;
    let mut bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| crate::api::error::ApiError::from(crate::domain::errors::DomainError::Validation {
            message: format!("Yükleme okunamadı: {e}"),
        }))?
    {
        match field.name().unwrap_or_default() {
            "entity_type" => entity_type = Some(field.text().await.unwrap_or_default()),
            "entity_id" => {
                entity_id = field.text().await.ok().and_then(|t| t.parse().ok());
            }
            "file" => {
                file_name = field.file_name().map(String::from);
                mime_type = Some(
                    field
                        .content_type()
                        .unwrap_or("application/octet-stream")
                        .to_string(),
                );
                bytes = Some(field.bytes().await.map_err(|_| {
                    crate::api::error::ApiError::from(
                        crate::domain::errors::DomainError::Validation {
                            message: "Dosya okunamadı.".into(),
                        },
                    )
                })?.to_vec());
            }
            _ => {}
        }
    }

    let entity_type = entity_type.ok_or_else(|| invalid("entity_type zorunludur."))?;
    let entity_id = entity_id.ok_or_else(|| invalid("entity_id zorunludur."))?;
    let file_name = file_name.ok_or_else(|| invalid("file zorunludur."))?;
    let mime_type = mime_type.unwrap_or_else(|| "application/octet-stream".into());
    let bytes = bytes.ok_or_else(|| invalid("file zorunludur."))?;

    let attachment = service::upload(
        &state.pool,
        state.storage.as_ref(),
        &auth.user,
        service::UploadInput {
            entity_type: &entity_type,
            entity_id,
            file_name: &file_name,
            mime_type: &mime_type,
            bytes,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(attachment)))
}

fn invalid(msg: &str) -> crate::api::error::ApiError {
    crate::api::error::ApiError::from(crate::domain::errors::DomainError::Validation {
        message: msg.into(),
    })
}

#[derive(Debug, Deserialize)]
pub struct AttachmentFilter {
    pub entity_type: String,
    pub entity_id: Uuid,
}

/// GET /api/v1/attachments?entity_type=&entity_id=
pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(filter): Query<AttachmentFilter>,
) -> ApiResult<Json<Vec<Attachment>>> {
    Ok(Json(
        service::list_attachments(&state.pool, &auth.user, &filter.entity_type, filter.entity_id)
            .await?,
    ))
}

/// GET /api/v1/attachments/{id}/download
pub async fn download(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(attachment_id): Path<Uuid>,
) -> ApiResult<Response> {
    let (attachment, bytes) =
        service::download(&state.pool, state.storage.as_ref(), &auth.user, attachment_id).await?;
    let response = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, attachment.mime_type.clone()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", attachment.file_name),
            ),
        ],
        crate::infrastructure::storage::bytes_to_body(bytes),
    )
        .into_response();
    Ok(response)
}

/// DELETE /api/v1/attachments/{id} — soft
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(attachment_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    service::delete(&state.pool, &auth.user, attachment_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
