use form_urlencoded;
use serde_json::Value;
use std::env;
use wstd::http::body::Body;
use wstd::http::{BodyExt, Client, Error, Method, Request, Response, StatusCode, Uri};
use wstd::time::{Duration, Instant};

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
        "/wait" => wait(req).await,
        "/echo" => echo(req).await,
        "/echo-headers" => echo_headers(req).await,
        "/api/chat-completion" => chat_completion(req).await,
        "/api/get-balance" => get_balance(req).await,
        "/api/get-transfer" => get_transfer(req).await,
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

fn build_json_response(status: StatusCode, body: String) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body.into())
        .unwrap()
}

fn solana_rpc_url() -> String {
    env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "https://api.devnet.solana.com".to_string())
}

async fn send_json(uri: Uri, payload: String) -> Result<String, String> {
    let mut request_builder = Request::builder()
        .method(Method::POST)
        .uri(uri.clone())
        .header("content-type", "application/json");

    if uri.authority().map(|auth| auth.as_str()) == Some("api.openai.com") {
        if let Ok(api_key) = env::var("OPENAI_API_KEY") {
            request_builder = request_builder.header("authorization", format!("Bearer {api_key}"));
        }
    }

    let request = request_builder
        .body(Body::from(payload))
        .map_err(|err| format!("failed to build request: {err}"))?;

    let response = Client::new()
        .send(request)
        .await
        .map_err(|err| format!("http request failed: {err}"))?;

    let collected = response
        .into_body()
        .into_boxed_body()
        .collect()
        .await
        .map_err(|err| format!("http response read failed: {err}"))?;

    Ok(String::from_utf8_lossy(collected.to_bytes().as_ref()).to_string())
}

async fn send_json_rpc(payload: String) -> Result<String, String> {
    let rpc_url: Uri = solana_rpc_url()
        .parse()
        .map_err(|err| format!("invalid SOLANA_RPC_URL: {err}"))?;

    send_json(rpc_url, payload).await
}

async fn get_balance(req: Request<Body>) -> Result<Response<Body>, Error> {
    let address = match query_param(&req, "addr") {
        Some(address) => address,
        None => {
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

    Ok(build_json_response(StatusCode::OK, text))
}

async fn get_transfer(req: Request<Body>) -> Result<Response<Body>, Error> {
    let Some(to) = query_param(&req, "addr") else {
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
            let body =
                serde_json::json!({ "error": format!("solana rpc blockhash none") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };
    let height: i64 = match json["result"]["value"]["lastValidBlockHeight"].as_i64() {
        Some(value) => value,
        None => {
            let body =
                serde_json::json!({ "error": format!("solana rpc block hight none") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let seed = "persistent keys arrive soon";
    let transfer = bindings::local::app::helpers_interface::transfer_from_seed(
        &seed, &to, 1_000_000, &blockhash, height,
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

    Ok(build_json_response(StatusCode::OK, text))
}

async fn chat_completion(req: Request<Body>) -> Result<Response<Body>, Error> {
    if let Some(api_key) = query_param(&req, "apiKey") {
        env::set_var("OPENAI_API_KEY", api_key);
    }

    let Some(message) = query_param(&req, "message") else {
        return bad_request("missing query param `message`\n").await;
    };

    let model = query_param(&req, "model").unwrap_or_else(|| "gpt-4o-mini".to_string());
    let payload = serde_json::json!({
        "model": model,
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

async fn bad_request(message: &str) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(message.to_string().into())?)
}

async fn hi(_req: Request<Body>) -> Result<Response<Body>, Error> {
    Ok(Response::new("hi!\n".to_string().into()))
}

async fn not_found(_req: Request<Body>) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("404\n".to_string().into())?)
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
        .body(body.into())?)
}
