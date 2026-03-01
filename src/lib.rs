use form_urlencoded;
use wstd::http::body::{BodyForthcoming, IncomingBody};
use wstd::http::server::{Finished, Responder};
use wstd::http::{IntoBody, Request, Response, StatusCode};
use wstd::io::{copy, AsyncWrite};
use wstd::time::{Duration, Instant};

mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "app",
    });
}

#[wstd::http_server]
async fn main(req: Request<IncomingBody>, res: Responder) -> Finished {
    match req.uri().path() {
        "/" => hi(req, res).await,
        "/wait" => wait(req, res).await,
        "/echo" => echo(req, res).await,
        "/echo-headers" => echo_headers(req, res).await,
        "/api/chat-completion" => chat_completion(req, res).await,
        "/api/get-balance" => get_balance(req, res).await,
        _ => not_found(req, res).await,
    }
}

fn query_param(req: &Request<IncomingBody>, key: &str) -> Option<String> {
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

async fn get_balance(req: Request<IncomingBody>, responder: Responder) -> Finished {
    let rpc = "https://api.devnet.solana.com";
    let Some(input) = query_param(&req, "addr") else {
        return bad_request(responder, "missing query param `addr`\n").await;
    };

    let handle = bindings::local::app::helpers_interface::get_balance(&rpc, &input);
    let output = poll_until_ready(handle).await;
    let is_error = is_error_json(&output);

    let response = Response::builder()
        .status(if is_error {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::OK
        })
        .header("content-type", "application/json")
        .body(output.into_body())
        .unwrap();
    responder.respond(response).await
}

async fn chat_completion(req: Request<IncomingBody>, responder: Responder) -> Finished {
    let Some(api_key) = query_param(&req, "apiKey") else {
        return bad_request(responder, "missing query param `apiKey`\n").await;
    };

    let Some(message) = query_param(&req, "message") else {
        return bad_request(responder, "missing query param `message`\n").await;
    };

    let model = query_param(&req, "model").unwrap_or_else(|| "gpt-4o-mini".to_string());
    let payload = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": message,
            }
        ]
    })
    .to_string();

    let handle = bindings::local::app::helpers_interface::chat_completion(&api_key, &payload);
    let output = poll_until_ready(handle).await;
    let is_error = is_error_json(&output);

    let response = Response::builder()
        .status(if is_error {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::OK
        })
        .header("content-type", "application/json")
        .body(output.into_body())
        .unwrap();
    responder.respond(response).await
}

async fn poll_until_ready(handle: i64) -> String {
    loop {
        let output = bindings::local::app::helpers_interface::poll(handle);
        if output != "delay" {
            return output;
        }
        wstd::task::sleep(Duration::from_millis(100)).await;
    }
}

fn is_error_json(output: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(output) else {
        return false;
    };

    matches!(
        value,
        serde_json::Value::Object(ref map)
            if map.len() == 1 && matches!(map.get("error"), Some(serde_json::Value::String(_)))
    )
}

async fn bad_request(responder: Responder, message: &str) -> Finished {
    let res = Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(message.to_string().into_body())
        .unwrap();
    responder.respond(res).await
}

async fn hi(_req: Request<IncomingBody>, res: Responder) -> Finished {
    res.respond(Response::new("hi!\n".into_body())).await
}

async fn not_found(_req: Request<IncomingBody>, responder: Responder) -> Finished {
    let res = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("404\n".into_body())
        .unwrap();
    responder.respond(res).await
}

async fn wait(_req: Request<IncomingBody>, res: Responder) -> Finished {
    // Get the time now
    let now = Instant::now();

    // Sleep for one second.
    wstd::task::sleep(Duration::from_secs(1)).await;

    // Compute how long we slept for.
    let elapsed = Instant::now().duration_since(now).as_millis();

    // To stream data to the response body, use `res::start_response`.
    let mut body = res.start_response(Response::new(BodyForthcoming));
    let result = body
        .write_all(format!("slept for {elapsed} millis\n").as_bytes())
        .await;
    Finished::finish(body, result, None)
}

async fn echo(mut req: Request<IncomingBody>, res: Responder) -> Finished {
    // Stream data from the req body to the response body.
    let mut body = res.start_response(Response::new(BodyForthcoming));
    let result = copy(req.body_mut(), &mut body).await;
    Finished::finish(body, result, None)
}

async fn echo_headers(req: Request<IncomingBody>, responder: Responder) -> Finished {
    let (parts, _body) = req.into_parts();
    let mut headers_json: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for (name, value) in parts.headers.iter() {
        let key = name.as_str().to_string();
        let val = match value.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(value.as_bytes()).to_string(),
        };
        headers_json.entry(key).or_default().push(val);
    }

    let body = serde_json::to_string_pretty(&headers_json).unwrap_or_else(|_| "{}".to_string());
    let res = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(body.into_body())
        .unwrap();
    responder.respond(res).await
}
