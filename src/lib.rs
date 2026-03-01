use std::env;
use form_urlencoded;
use serde_json::Value;
use wstd::http::body::Body;
use wstd::http::{BodyExt, Client, Error, Method, Request, Response, StatusCode, Uri};

mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "app",
    });
}

#[wstd::http_server]
async fn main(req: Request<Body>) -> Result<Response<Body>, Error> {
    match req.uri().path() {
        "/" => hi(req).await,
        "/api/chat-completion" => chat_completion(req).await,
        "/api/get-balance" => get_balance(req).await,
        "/api/get-transfer" => get_transfer(req).await,
        _ => not_found(req).await,
    }
}

async fn hi(_req: Request<Body>) -> Result<Response<Body>, Error> {
    Ok(Response::new("hi!\n".to_string().into()))
}

async fn not_found(_req: Request<Body>) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("404\n".to_string().into())?)
}

async fn bad_request(message: &str) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(message.to_string().into())?)
}

fn build_json_response(status: StatusCode, body: String) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body.into())
        .unwrap()
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

async fn send_json(uri: Uri, payload: String) -> Result<String, String> {
    let mut request_builder = Request::builder()
        .method(Method::POST)
        .uri(uri.clone())
        .header("content-type", "application/json");

    if uri.authority().map(|auth| auth.as_str()) == Some("api.openai.com") {
        if let Ok(openai_key) = env::var("openai_key") {
            request_builder = request_builder.header("authorization", format!("Bearer {openai_key}"));
        }
    }

    let request = request_builder
        .body(Body::from(payload))
        .map_err(|err| format!("http build error: {uri} {err}"))?;

    let response = Client::new()
        .send(request)
        .await
        .map_err(|err| format!("http send error: {uri} {err}"))?;

    let collected = response
        .into_body()
        .into_boxed_body()
        .collect()
        .await
        .map_err(|err| format!("http read error: {uri} {err}"))?;

    Ok(String::from_utf8_lossy(collected.to_bytes().as_ref()).to_string())
}

async fn send_json_rpc(payload: String) -> Result<String, String> {
    let rpc_url: String = env::var("solana_net")
        .map_err(|_err| format!("invalid solana_net env"))?;

    let rpc_url: Uri = rpc_url
        .parse()
        .map_err(|_err| format!("invalid solana_net env"))?;

    send_json(rpc_url, payload).await
}

async fn get_balance(req: Request<Body>) -> Result<Response<Body>, Error> {
    let address = match query_param(&req, "addr") {
        // user address
        Some(address) => address,
        None => {
            // app address
            let seed = "persistent keys arrive soon";
            bindings::local::app::helpers_interface::address_from_seed(&seed)
        }
    };

    let payload = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getBalance",
        "params": [address]
    })
    .to_string();

    let text = match send_json_rpc(payload).await {
        Ok(text) => text,
        Err(err) => {
            let body = serde_json::json!({ "error": err }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    // todo: shape
    Ok(build_json_response(StatusCode::OK, text))
}

async fn get_transfer(req: Request<Body>) -> Result<Response<Body>, Error> {
    let Some(destination) = query_param(&req, "addr") else {
        return bad_request("missing query param `addr`\n").await;
    };

    let payload = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getLatestBlockhash",
        "params": [{"commitment": "confirmed"}]
    })
    .to_string();

    let text = match send_json_rpc(payload).await {
        Ok(text) => text,
        Err(err) => {
            let body = serde_json::json!({ "error": err }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let json: Value = serde_json::from_str(&text)?;
    let blockhash: String = match json["result"]["value"]["blockhash"].as_str() {
        Some(value) => value.to_string(),
        None => {
            let body = serde_json::json!({ "error": format!("bad blockhash") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };
    let height: i64 = match json["result"]["value"]["lastValidBlockHeight"].as_i64() {
        Some(value) => value,
        None => {
            let body = serde_json::json!({ "error": format!("bad lastValidBlockHeight") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let seed = "persistent keys arrive soon";
    let transfer = bindings::local::app::helpers_interface::transfer_from_seed(
        &seed, &destination, 1_000_000, &blockhash, height,
    );
    let mut transfer = transfer.split(',');
    let signed_tx = transfer.next().unwrap_or("empty");
    let _signature = transfer.next().unwrap_or("empty");

    let payload = serde_json::json!({
        "jsonrpc": "2.0", "id": 2,
        "method": "sendTransaction",
        "params": [signed_tx, {"preflightCommitment":"confirmed","encoding":"base64"}]
    })
    .to_string();

    let text = match send_json_rpc(payload).await {
        Ok(text) => text,
        Err(err) => {
            let body = serde_json::json!({ "error": err }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    // todo: shape
    Ok(build_json_response(StatusCode::OK, text))
}

async fn chat_completion(req: Request<Body>) -> Result<Response<Body>, Error> {
    let Some(message) = query_param(&req, "message") else {
        return bad_request("missing query param `message`\n").await;
    };

    let payload = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [{ "role": "user", "content": message }]
    })
    .to_string();

    let uri: Uri = "https://api.openai.com/v1/chat/completions"
        .parse()
        .unwrap();
    let text = match send_json(uri, payload).await {
        Ok(text) => text,
        Err(err) => {
            let body = serde_json::json!({ "error": err }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    Ok(build_json_response(StatusCode::OK, text))
}
