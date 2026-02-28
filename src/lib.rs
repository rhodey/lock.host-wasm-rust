use form_urlencoded;
use serde_json::Value;
use wstd::http::body::Body;
use wstd::http::{Error, Request, Response, StatusCode};
use wstd::time::{Duration, Instant};

mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "app",
        generate_all,
    });
}

#[wstd::http_server]
async fn main(req: Request<Body>) -> Result<Response<Body>, Error> {
    match req.uri().path() {
        "/" => hi(req).await,
        "/wait" => wait(req).await,
        "/echo" => echo(req).await,
        "/echo-headers" => echo_headers(req).await,
        "/api/chat-completion" => chat_completion(req).await,
        "/api/get-balance" => get_balance(req).await,
        _ => not_found(req).await,
    }
}

fn query_param(req: &Request<Body>, key: &str) -> Option<String> {
    req.uri().query().and_then(|query| {
        form_urlencoded::parse(query.as_bytes()).find_map(|(k, v)| {
            if k == key {
                Some(v.into_owned())
            } else {
                None
            }
        })
    })
}

fn read_json_body(stream: bindings::wasi::io::streams::InputStream) -> (StatusCode, String) {
    let mut bytes = Vec::new();
    let mut empty_reads = 0u32;
    loop {
        match stream.blocking_read(8_192) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    empty_reads += 1;
                    if !bytes.is_empty() && empty_reads >= 3 {
                        break;
                    }
                    if bytes.is_empty() && empty_reads >= 500 {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            serde_json::json!({ "error": "timed out reading helper stream" })
                                .to_string(),
                        );
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }

                empty_reads = 0;
                bytes.extend_from_slice(&chunk);
            }
            Err(bindings::wasi::io::streams::StreamError::Closed) => break,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::json!({ "error": "failed reading helper stream" }).to_string(),
                )
            }
        }
    }

    let body = match String::from_utf8(bytes) {
        Ok(body) => body,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({ "error": format!("invalid utf-8: {err}") }).to_string(),
            )
        }
    };

    let parsed: Value = match serde_json::from_str(&body) {
        Ok(parsed) => parsed,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({ "error": format!("invalid json: {err}") }).to_string(),
            )
        }
    };

    let is_error = parsed
        .as_object()
        .and_then(|obj| obj.get("error"))
        .map(|v| v.is_string())
        .unwrap_or(false);

    if is_error {
        (StatusCode::INTERNAL_SERVER_ERROR, body)
    } else {
        (StatusCode::OK, body)
    }
}

fn build_json_response(status: StatusCode, body: String) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body.into())
        .unwrap()
}

async fn get_balance(req: Request<Body>) -> Result<Response<Body>, Error> {
    let rpc = "https://api.devnet.solana.com";
    let Some(input) = query_param(&req, "addr") else {
        return bad_request("missing query param `addr`\n").await;
    };

    let output = bindings::local::app::helpers_interface::get_balance(&rpc, &input);
    let (status, body) = read_json_body(output);
    Ok(build_json_response(status, body))
}

async fn chat_completion(req: Request<Body>) -> Result<Response<Body>, Error> {
    let Some(_api_key) = query_param(&req, "apiKey") else {
        return bad_request("missing query param `apiKey`
").await;
    };

    let Some(message) = query_param(&req, "message") else {
        return bad_request("missing query param `message`
").await;
    };

    let model = query_param(&req, "model").unwrap_or_else(|| "gpt-4o-mini".to_string());
    let body = serde_json::json!({
        "message": message,
        "model": model,
        "error": "chat helper stream is currently unavailable; request was accepted by Rust handler"
    })
    .to_string();

    Ok(build_json_response(StatusCode::OK, body))
}

async fn bad_request(message: &str) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(message.to_string().into())
        .unwrap())
}

async fn hi(_req: Request<Body>) -> Result<Response<Body>, Error> {
    Ok(Response::new("hi!\n".into()))
}

async fn not_found(_req: Request<Body>) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("404\n".into())
        .unwrap())
}

async fn wait(_req: Request<Body>) -> Result<Response<Body>, Error> {
    let now = Instant::now();
    wstd::task::sleep(Duration::from_secs(1)).await;
    let elapsed = Instant::now().duration_since(now).as_millis();

    Ok(Response::new(
        format!("slept for {elapsed} millis\n").into(),
    ))
}

async fn echo(req: Request<Body>) -> Result<Response<Body>, Error> {
    let (_parts, body) = req.into_parts();
    Ok(Response::new(body))
}

async fn echo_headers(req: Request<Body>) -> Result<Response<Body>, Error> {
    let (parts, _body) = req.into_parts();
    let mut headers_json: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for (name, value) in &parts.headers {
        let key = name.as_str().to_string();
        let val = match value.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(value.as_bytes()).to_string(),
        };
        headers_json.entry(key).or_default().push(val);
    }

    let body = serde_json::to_string_pretty(&headers_json).unwrap_or_else(|_| "{}".to_string());
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(body.into())
        .unwrap())
}
