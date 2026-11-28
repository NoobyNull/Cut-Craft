//! The docs page at `/docs`: Swagger UI, carried inside PowerWhittle (no internet needed), showing
//! the description at [`DESCRIPTION`] so every call can be read about and tried in a browser.

use crate::ApiError;
use std::sync::Arc;

/// Where the description of every call is read from.
pub const DESCRIPTION: &str = "/api/v1/openapi.json";

/// How the page is set up: it reads [`DESCRIPTION`], lets calls be tried straight away, and
/// keeps a token typed into Authorize while the page stays open. No address bar: it shows
/// only PowerWhittle's own calls.
pub(crate) fn config() -> Arc<utoipa_swagger_ui::Config<'static>> {
    Arc::new(
        utoipa_swagger_ui::Config::new([DESCRIPTION])
            .try_it_out_enabled(true)
            .display_request_duration(true)
            .persist_authorization(true)
            .use_base_layout(),
    )
}

type Page = tiny_http::Response<std::io::Cursor<Vec<u8>>>;

/// The docs page's file at `path` (`/docs`, `/docs/`, `/docs/<file>`).
pub(crate) fn file(
    path: &str,
    config: &Arc<utoipa_swagger_ui::Config<'static>>,
) -> Result<Page, ApiError> {
    // the page's own files are found relative to it, so it must end in a slash
    if path == "/docs" {
        let to = tiny_http::Header::from_bytes(&b"Location"[..], &b"/docs/"[..])
            .expect("a fixed header is valid");
        return Ok(tiny_http::Response::from_data(Vec::new())
            .with_status_code(308)
            .with_header(to));
    }
    let tail = path.trim_start_matches("/docs/");
    let found = utoipa_swagger_ui::serve(tail, Arc::clone(config)).map_err(|e| ApiError {
        status: 500,
        message: format!("The docs page could not be made: {e}."),
    })?;
    let Some(found) = found else {
        return Err(ApiError::not_found(format!(
            "There is no {path}. The docs page is /docs/."
        )));
    };
    let kind = tiny_http::Header::from_bytes(&b"Content-Type"[..], found.content_type.as_bytes())
        .expect("a file type is a valid header");
    let mut page = tiny_http::Response::from_data(found.bytes.into_owned()).with_header(kind);
    if found.gzpipped {
        page.add_header(
            tiny_http::Header::from_bytes(&b"Content-Encoding"[..], &b"gzip"[..])
                .expect("a fixed header is valid"),
        );
    }
    Ok(page)
}
