//! GET/PATCH configuration boundary (management-api-v1 §1.13–1.14).
use super::session_api::{is_json, media_type_allows, poll_flag};
use super::{PeerAddr, Shared, json_error, json_response, method_not_allowed};
use crate::clock::timestamp;
use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::{Value, json};
use std::sync::Arc;
use vnc_relay_app::management::config::{
    ConfigActor, ConfigChange, ConfigEditError, ConfigPatch, ConfigReloadKind, ConfigValue,
    ConfigView, valid_revision,
};
use vnc_relay_app::management::session::AuthorizedAdmin;
use vnc_relay_app::reload::{ConfigFileState, ReloadResult, ReloadTrigger};
use vnc_relay_app::settings::ConfigDiagnostic;

const PATCH_BODY_LIMIT: usize = 65_536;

pub(super) async fn answer(
    shared: &Shared,
    peer: &PeerAddr,
    admin: &mut AuthorizedAdmin,
    parts: Parts,
    body: Body,
) -> Response {
    match parts.method {
        Method::GET => {
            let Some(poll) = poll_flag(&parts.headers) else {
                return invalid_request();
            };
            if !media_type_allows(body, &parts.headers).await {
                return unsupported_media_type();
            }
            let config = Arc::clone(&shared.config);
            match tokio::task::spawn_blocking(move || config.get()).await {
                Ok(Ok(view)) => {
                    if !poll {
                        shared.auth.touch(admin);
                    }
                    json_response(StatusCode::OK, &view_body(view))
                }
                _ => internal_error(),
            }
        }
        Method::PATCH => {
            if parts.headers.contains_key("x-relay-poll") {
                return invalid_request();
            }
            if !is_json(&parts.headers) {
                return unsupported_media_type();
            }
            let bytes = match axum::body::to_bytes(body, PATCH_BODY_LIMIT).await {
                Ok(bytes) => bytes,
                Err(_) => return invalid_request(),
            };
            let Some(request) = patch_body(&bytes) else {
                return invalid_request();
            };
            let actor = ConfigActor {
                admin_account: admin.session().account.clone(),
                peer_ip: peer.ip,
            };
            let config = Arc::clone(&shared.config);
            // Dropping this response future does not cancel a started blocking
            // transaction. In particular, replace is always followed by commit.
            match tokio::task::spawn_blocking(move || config.patch(actor, request)).await {
                Ok(Ok(result)) => {
                    shared.auth.touch(admin);
                    json_response(
                        StatusCode::OK,
                        &json!({ "as_of":timestamp(result.as_of), "result":"applied", "revision":result.revision, "applied":result.applied }),
                    )
                }
                Ok(Err(error)) => edit_error(error),
                Err(_) => internal_error(),
            }
        }
        _ => method_not_allowed("GET, PATCH", "이 경로는 GET과 PATCH 요청만 받는다."),
    }
}
fn patch_body(bytes: &[u8]) -> Option<ConfigPatch> {
    let root: Value = serde_json::from_slice(bytes).ok()?;
    let revision = root.get("revision")?.as_str()?;
    if !valid_revision(revision) {
        return None;
    }
    let object = root.get("changes")?.as_object()?;
    if object.is_empty() {
        return None;
    }
    let changes = object
        .iter()
        .map(|(key, value)| {
            let value = if let Some(integer) = value.as_i64() {
                ConfigValue::Integer(integer)
            } else if let Some(integer) = value.as_u64() {
                ConfigValue::Integer(i64::try_from(integer).unwrap_or(i64::MAX))
            } else {
                ConfigValue::Strings(
                    value
                        .as_array()?
                        .iter()
                        .map(|value| value.as_str().map(str::to_owned))
                        .collect::<Option<Vec<_>>>()?,
                )
            };
            Some(ConfigChange {
                key: key.clone(),
                value,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ConfigPatch {
        revision: revision.to_owned(),
        changes,
    })
}
fn value_body(value: ConfigValue) -> Value {
    match value {
        ConfigValue::Bool(value) => json!(value),
        ConfigValue::Integer(value) => json!(value),
        ConfigValue::String(value) => json!(value),
        ConfigValue::Strings(value) => json!(value),
        ConfigValue::Null => Value::Null,
    }
}
fn diagnostics_body(diagnostics: Vec<ConfigDiagnostic>) -> Value {
    Value::Array(
        diagnostics
            .into_iter()
            .map(|d| json!({"code":d.error.code(),"key":d.key,"message":d.message}))
            .collect(),
    )
}
fn view_body(view: ConfigView) -> Value {
    let items = view.items.into_iter().map(|item| {
        let mut body = json!({"key":item.key,"value":value_body(item.value),"reload":match item.reload { ConfigReloadKind::Dynamic=>"dynamic", ConfigReloadKind::Restart=>"restart" },"editable":item.editable});
        if let (Some(min),Some(max)) = (item.min,item.max) { body["min"]=json!(min); body["max"]=json!(max); }
        body
    }).collect::<Vec<_>>();
    let last_reload = view.last_reload.map(|last| json!({"at":timestamp(last.at),"result":match last.result {ReloadResult::Applied=>"applied",ReloadResult::Rejected=>"rejected",ReloadResult::RestartRequired=>"restart_required"},"trigger":match last.trigger {ReloadTrigger::FileWatch=>"file_watch",ReloadTrigger::ConfigEdit=>"config_edit"},"config_keys":last.config_keys}));
    json!({"as_of":timestamp(view.as_of),"revision":view.revision,"edit_enabled":view.edit_enabled,
        "file":{"state":match view.file.state { ConfigFileState::InEffect=>"in_effect",ConfigFileState::Pending=>"pending",ConfigFileState::RestartRequired=>"restart_required",ConfigFileState::Invalid=>"invalid",ConfigFileState::Unreadable=>"unreadable" },"config_keys":view.file.config_keys,"diagnostics":diagnostics_body(view.file.diagnostics)},
        "last_reload":last_reload,"items":items})
}
fn edit_error(error: ConfigEditError) -> Response {
    let (status, message) = match &error {
        ConfigEditError::InvalidRequest => (
            StatusCode::BAD_REQUEST,
            "편집 요청의 형식이나 대상이 맞지 않다.",
        ),
        ConfigEditError::ConfigInvalid { .. } => {
            (StatusCode::BAD_REQUEST, "후보 설정이 유효하지 않다.")
        }
        ConfigEditError::EditDisabled => (StatusCode::FORBIDDEN, "설정 편집 기능이 꺼져 있다."),
        ConfigEditError::Conflict => (
            StatusCode::CONFLICT,
            "설정 파일이 바뀌었다. 다시 조회해야 한다.",
        ),
        ConfigEditError::RestartPending { .. } => (
            StatusCode::CONFLICT,
            "재시작이 필요한 파일 변경이 남아 있다.",
        ),
        ConfigEditError::FileInvalid { .. } => {
            (StatusCode::CONFLICT, "현재 설정 파일을 사용할 수 없다.")
        }
        ConfigEditError::WriteFailed => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "설정 파일을 교체하지 못했다. 파일과 실행값은 바뀌지 않았다.",
        ),
        ConfigEditError::Internal => return internal_error(),
    };
    let mut body = json!({"error":{"code":error.code(),"message":message}});
    match error {
        ConfigEditError::ConfigInvalid { diagnostics }
        | ConfigEditError::FileInvalid { diagnostics } => {
            body["error"]["diagnostics"] = diagnostics_body(diagnostics)
        }
        ConfigEditError::RestartPending { config_keys } => {
            body["error"]["config_keys"] = json!(config_keys)
        }
        _ => {}
    }
    json_response(status, &body)
}
fn invalid_request() -> Response {
    json_error(
        StatusCode::BAD_REQUEST,
        "invalid_request",
        "편집 요청의 형식이나 대상이 맞지 않다.",
    )
}
fn unsupported_media_type() -> Response {
    json_error(
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "invalid_request",
        "요청 본문은 application/json이어야 한다.",
    )
}
fn internal_error() -> Response {
    json_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal",
        "설정 요청을 처리하지 못했다.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// spec: management.md:MGT-036 partial
    #[test]
    fn unsigned_json_integer_overflow_reaches_configuration_range_validation() {
        let revision = format!("sha256:{}", "0".repeat(64));
        let body = format!(
            r#"{{"revision":"{revision}","changes":{{"limits.waiting":18446744073709551615}}}}"#
        );
        let request = patch_body(body.as_bytes())
            .unwrap_or_else(|| panic!("u64 integer rejected as a type error"));
        assert_eq!(request.changes[0].value, ConfigValue::Integer(i64::MAX));
    }

    /// spec: management.md:MGT-037 partial
    #[test]
    fn integer_literals_beyond_u64_are_request_type_errors() {
        let revision = format!("sha256:{}", "0".repeat(64));
        for number in ["18446744073709551616", "1e23", "1.5"] {
            let body =
                format!(r#"{{"revision":"{revision}","changes":{{"limits.waiting":{number}}}}}"#);
            assert!(patch_body(body.as_bytes()).is_none());
        }
    }
}
