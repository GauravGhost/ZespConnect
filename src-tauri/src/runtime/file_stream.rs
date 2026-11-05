use tauri::http::{header, Request, Response, StatusCode};
use tauri::{AppHandle, Manager, UriSchemeContext, UriSchemeResponder};

use crate::error::Result;
use crate::services::file_stream::{self, ByteRange, StreamTarget};
use crate::state::AppState;

type HttpResponse = Response<Vec<u8>>;

/// The protocol handler; each request is answered off the webview's thread.
pub fn handle(
    context: UriSchemeContext<'_, tauri::Wry>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = context.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        let response = respond(&app, &request)
            .await
            .unwrap_or_else(|error| plain(StatusCode::BAD_GATEWAY, error.to_string()));
        responder.respond(response);
    });
}

async fn respond(app: &AppHandle, request: &Request<Vec<u8>>) -> Result<HttpResponse> {
    let StreamTarget { serial, remote } = file_stream::parse_target(request.uri().path())?;
    let tools = app.state::<AppState>().tools();
    let size = file_stream::file_size(&tools, &serial, &remote).await?;

    let range_header = request
        .headers()
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    let (status, range) = match range_header {
        Some(value) => match file_stream::range_for(value, size) {
            Some(range) => (StatusCode::PARTIAL_CONTENT, range),
            None => return Ok(unsatisfiable(size)),
        },
        None if size > file_stream::MAX_WHOLE_FILE => {
            return Ok(plain(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Too large to preview".into(),
            ));
        }
        None => (StatusCode::OK, ByteRange::whole(size)),
    };

    let body = if size == 0 {
        Vec::new()
    } else {
        file_stream::read_range(&tools, &serial, &remote, range).await?
    };
    let mut response = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, file_stream::mime_type(&remote))
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CONTENT_LENGTH, body.len());
    if status == StatusCode::PARTIAL_CONTENT {
        let end = range.start + (body.len() as u64).saturating_sub(1);
        response = response.header(
            header::CONTENT_RANGE,
            format!("bytes {}-{end}/{size}", range.start),
        );
    }
    Ok(response.body(body).unwrap_or_default())
}

fn unsatisfiable(size: u64) -> HttpResponse {
    Response::builder()
        .status(StatusCode::RANGE_NOT_SATISFIABLE)
        .header(header::CONTENT_RANGE, format!("bytes */{size}"))
        .body(Vec::new())
        .unwrap_or_default()
}

fn plain(status: StatusCode, message: String) -> HttpResponse {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(message.into_bytes())
        .unwrap_or_default()
}
