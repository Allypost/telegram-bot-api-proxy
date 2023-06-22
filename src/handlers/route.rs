use std::path::PathBuf;

use axum::body::{Body, StreamBody};
use axum::extract::State;
use axum::http::{header, Request, StatusCode};

use axum::{extract::Path, response::IntoResponse};
use hyper::body::HttpBody;
use log::{debug, error, trace};
use serde_json::Value;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::config::CONFIG;

use super::HttpClient;

pub async fn handle_serve_file(
    Path((bot_id, file_path)): Path<(String, PathBuf)>,
) -> impl IntoResponse {
    let base_path = &CONFIG.base_path;
    trace!("base_path: {path:?}", path = &base_path);

    let local_file_path = base_path.join(&bot_id).join(&file_path);
    trace!("local_file_path: {path:?}", path = &local_file_path);
    let local_file_path = local_file_path.canonicalize().map_err(|e| {
        debug!("canonicalize error: {error:?}", error = &e);
        StatusCode::NOT_FOUND
    })?;
    trace!(
        "canonicalized local_file_path: {path:?}",
        path = &local_file_path
    );

    if !local_file_path.starts_with(base_path) {
        return Err(StatusCode::NOT_FOUND);
    }

    let file = File::open(local_file_path).await.map_err(|e| {
        debug!("open error: {error:?}", error = &e);
        StatusCode::NOT_FOUND
    });
    trace!("file: {file:?}");
    let file = file?;

    let _metadata = file.metadata().await.map_err(|e| {
        debug!("metadata error: {error:?}", error = &e);
        StatusCode::NOT_FOUND
    })?;

    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    Ok((StatusCode::OK, body))
}

pub async fn handle_post_file(
    Path(bot_id): Path<String>,
    State(client): State<HttpClient>,
    mut req: Request<Body>,
) -> impl IntoResponse {
    let uri = {
        let path = req.uri().path();
        let path_query = req
            .uri()
            .path_and_query()
            .map_or(path, axum::http::uri::PathAndQuery::as_str)
            .trim_start_matches('/');

        let uri = format!("{base}{path_query}", base = CONFIG.proxy_to);

        trace!("getting response from {uri}");

        uri
    };

    let mut resp = {
        *req.uri_mut() = uri.parse().unwrap();
        let resp = client.request(req).await.map_err(|e| {
            error!("client error: {}", e);
            (StatusCode::BAD_GATEWAY, e.to_string())
        });

        trace!("resp from remote: {resp:?}", resp = &resp);

        match resp {
            Ok(resp) if resp.status() == StatusCode::OK => resp,
            _ => return resp.into_response(),
        }
    };

    let mut resp_data = {
        let resp_body = resp.body_mut().data().await;
        let Some(Ok(resp_body)) = resp_body else {
            return resp.into_response();
        };
        let resp_body = resp_body.to_vec();
        let Ok(Value::Object(resp_body)) = serde_json::from_slice::<Value>(&resp_body) else {
            return resp.into_response();
        };

        trace!("resp_body: {resp_body:?}", resp_body = &resp_body);

        match resp_body.get("ok") {
            Some(Value::Bool(true)) => {}
            _ => return resp.into_response(),
        }

        resp_body
    };

    let Some(Value::Object(result)) = resp_data.get("result") else {
        return resp.into_response();
    };
    let Some(Value::String(file_path)) = result.get("file_path") else {
        return resp.into_response();
    };

    let split = file_path.split_once(&bot_id);

    trace!("split result: {split:?}", split = &split);

    let Some((_, real_path)) = split else {
        return resp.into_response();
    };

    let mut new_result = result.clone();
    new_result.insert(
        "file_path".to_string(),
        Value::String(real_path.trim_start_matches('/').to_string()),
    );
    resp_data.insert("result".to_string(), Value::Object(new_result));
    let new_body = serde_json::to_vec(&resp_data).expect("failed to serialize response body");

    let headers = resp.headers_mut();

    headers.insert(header::CONTENT_LENGTH, new_body.len().into());

    *resp.body_mut() = Body::from(new_body);

    resp.into_response()
}

pub async fn handle_proxy(
    State(client): State<HttpClient>,
    mut req: Request<Body>,
) -> impl IntoResponse {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map_or(path, axum::http::uri::PathAndQuery::as_str)
        .trim_start_matches('/');
    let uri = format!("{base}{path_query}", base = CONFIG.proxy_to);
    trace!("proxying /{path_query} to {uri}");

    *req.uri_mut() = uri.parse().unwrap();

    client
        .request(req)
        .await
        .map_err(|e| {
            error!("client error: {}", e);
            (StatusCode::BAD_GATEWAY, e.to_string())
        })
        .into_response()
}
