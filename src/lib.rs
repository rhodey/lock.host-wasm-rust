use form_urlencoded;
use serde_json::Value;
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

async fn get_balance(req: Request<Body>) -> Result<Response<Body>, Error> {
    // let rpc = "https://api.devnet.solana.com";
    let Some(address) = query_param(&req, "addr") else {
        return bad_request("missing query param `addr`\n").await;
    };

    let rpc_url: Uri = "https://api.devnet.solana.com".parse().unwrap();
    let payload = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getBalance",
        "params": [address]
    })
    .to_string();

    let rpc_request = Request::builder()
        .method(Method::POST)
        .uri(rpc_url)
        .header("content-type", "application/json")
        .body(Body::from(payload))
        .unwrap();

    let response = match Client::new().send(rpc_request).await {
        Ok(resp) => resp,
        Err(err) => {
            let body = serde_json::json!({ "error": format!("solana rpc failed: {err}") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let status = response.status();
    let collected = match response.into_body().into_boxed_body().collect().await {
        Ok(body) => body,
        Err(err) => {
            let body = serde_json::json!({ "error": format!("solana rpc read failed: {err}") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let text = String::from_utf8_lossy(collected.to_bytes().as_ref()).to_string();
    Ok(build_json_response(status, text))
}

async fn get_transfer(req: Request<Body>) -> Result<Response<Body>, Error> {
    let seed = "persistent keys arrive soon";
    let to = "CFf6SMjR3eNKR7me9CGHhRNE1SwSQaPi5r4MWZQFGB2W";

    let from = bindings::local::app::helpers_interface::address_from_seed(&seed);
    println!("from => {from}");
    println!("to => {to}");

    let rpc_url: Uri = "https://api.devnet.solana.com".parse().unwrap();
    let payload = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getLatestBlockhash",
        "params": [{"commitment": "confirmed"}]
    })
    .to_string();

    let rpc_request = Request::builder()
        .method(Method::POST)
        .uri(rpc_url)
        .header("content-type", "application/json")
        .body(Body::from(payload))
        .unwrap();

    let response = match Client::new().send(rpc_request).await {
        Ok(resp) => resp,
        Err(err) => {
            let body = serde_json::json!({ "error": format!("solana rpc failed: {err}") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let collected = match response.into_body().into_boxed_body().collect().await {
        Ok(body) => body,
        Err(err) => {
            let body = serde_json::json!({ "error": format!("solana rpc read failed: {err}") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let text = String::from_utf8_lossy(collected.to_bytes().as_ref()).to_string();
    println!("rpc => {text}");

    let v: Value = serde_json::from_str(&text)?;

    let blockhash: String = match v["result"]["value"]["blockhash"].as_str() {
        Some(value) => value.to_string(),
        None => {
            let body = serde_json::json!({ "error": format!("solana rpc blockhash none") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }

    };
    println!("hash => {blockhash}");

    let height: i64 = match v["result"]["value"]["lastValidBlockHeight"].as_i64() {
        Some(value) => value,
        None => {
            let body = serde_json::json!({ "error": format!("solana rpc block hight none") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }

    };
    println!("height => {height}");

    let transfer = bindings::local::app::helpers_interface::transfer_from_seed(&seed, &to, 1_000_000, &blockhash, height);
    println!("transfer => {transfer}");

    let mut transfer = transfer.split(',');
    let signed_tx = transfer.next().unwrap_or("empty");
    let signature = transfer.next().unwrap_or("empty");

    let rpc_url: Uri = "https://api.devnet.solana.com".parse().unwrap();
    let payload = serde_json::json!({
        "jsonrpc": "2.0", "id": 2,
        "method": "sendTransaction",
        "params": [signed_tx, {"preflightCommitment":"confirmed","encoding":"base64"}]
    })
    .to_string();

    let rpc_request = Request::builder()
        .method(Method::POST)
        .uri(rpc_url)
        .header("content-type", "application/json")
        .body(Body::from(payload))
        .unwrap();

    let response = match Client::new().send(rpc_request).await {
        Ok(resp) => resp,
        Err(err) => {
            let body = serde_json::json!({ "error": format!("solana rpc2 failed: {err}") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let status = response.status();
    let collected = match response.into_body().into_boxed_body().collect().await {
        Ok(body) => body,
        Err(err) => {
            let body = serde_json::json!({ "error": format!("solana rpc2 read failed: {err}") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let text = String::from_utf8_lossy(collected.to_bytes().as_ref()).to_string();
    println!("rpc2 => {text}");

    Ok(build_json_response(status, text))
}

async fn chat_completion(req: Request<Body>) -> Result<Response<Body>, Error> {
    let Some(api_key) = query_param(&req, "apiKey") else {
        return bad_request("missing query param `apiKey`\n").await;
    };

    let Some(message) = query_param(&req, "message") else {
        return bad_request("missing query param `message`\n").await;
    };

    let model = query_param(&req, "model").unwrap_or_else(|| "gpt-4o-mini".to_string());
    let payload = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": message }]
    })
    .to_string();

    let request = Request::builder()
        .method(Method::POST)
        .uri("https://api.openai.com/v1/chat/completions")
        .header("authorization", format!("Bearer {api_key}"))
        .header("content-type", "application/json")
        .body(Body::from(payload))
        .unwrap();

    let response = match Client::new().send(request).await {
        Ok(resp) => resp,
        Err(err) => {
            let body = serde_json::json!({ "error": format!("openai http failed: {err}") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let status = response.status();
    let collected = match response.into_body().into_boxed_body().collect().await {
        Ok(body) => body,
        Err(err) => {
            let body = serde_json::json!({ "error": format!("openai http read failed: {err}") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let text = String::from_utf8_lossy(collected.to_bytes().as_ref()).to_string();
    Ok(build_json_response(status, text))
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
