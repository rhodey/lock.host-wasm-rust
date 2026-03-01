use form_urlencoded;
use serde_json::Value;
use std::env;
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
    // todo: wallet + joke
    match req.uri().path() {
        "/" => hi(req).await,
        "/api/balance" => get_balance(req).await,
        "/api/joke" => get_joke(req).await,
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
            request_builder =
                request_builder.header("authorization", format!("Bearer {openai_key}"));
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
    let rpc_url: String =
        env::var("solana_net").map_err(|_err| format!("invalid solana_net env"))?;

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

async fn get_joke(req: Request<Body>) -> Result<Response<Body>, Error> {
    let Some(message) = query_param(&req, "message") else {
        return bad_request("missing query param `message`\n").await;
    };
    let Some(destination) = query_param(&req, "addr") else {
        return bad_request("missing query param `addr`\n").await;
    };

    let payload = serde_json::json!({
        "model": "gpt-4o-mini",
        "tools": [{
            "type": "function",
            "function": {
                "name": "record_if_joke_is_funny",
                "description": "Record if joke is funny",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "thoughts": { "type": "string" },
                        "decision": {
                            "type": "string",
                            "enum": ["funny", "not"],
                        },
                    },
                    "required": ["thoughts", "decision"],
                    "additionalProperties": false,
                },
                "strict": true,
            },
        }],
        "tool_choice": {
            "type": "function",
            "function": {
                "name": "record_if_joke_is_funny",
            },
        },
        "messages": [
            { "role": "system", "content": "You are to decide if a joke is funny or not" },
            { "role": "user", "content": message },
        ]
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
    println!("text =>> {text}");

    let json: Value = serde_json::from_str(&text)?;

    let tool_call: String = match json["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"].as_str() {
        Some(value) => value.to_string(),
        None => {
            let body = serde_json::json!({ "error": format!("bad tool_call") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    println!("tool_call =>> {tool_call}");
    let tool_call: Value = serde_json::from_str(&tool_call)?;

    let thoughts: String = match tool_call["thoughts"].as_str() {
        Some(value) => value.to_string(),
        None => {
            let body = serde_json::json!({ "error": format!("bad tool_call thoughts") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };
    println!("thoughts =>> {thoughts}");

    let decision: String = match tool_call["decision"].as_str() {
        Some(value) => value.to_string(),
        None => {
            let body = serde_json::json!({ "error": format!("bad tool_call decision") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };
    println!("decision =>> {decision}");

    if decision != "funny" {
        let body = serde_json::json!({ "thoughts": thoughts }).to_string();
        return Ok(build_json_response(StatusCode::OK, body))
    }

    let seed = "persistent keys arrive soon";
    let from = bindings::local::app::helpers_interface::address_from_seed(&seed);
    let body = serde_json::json!({ "from": from, "to": destination, "thoughts": thoughts }).to_string();
    Ok(build_json_response(StatusCode::OK, body))

    /*
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
            let body =
                serde_json::json!({ "error": format!("bad lastValidBlockHeight") }).to_string();
            return Ok(build_json_response(StatusCode::INTERNAL_SERVER_ERROR, body));
        }
    };

    let seed = "persistent keys arrive soon";
    let transfer = bindings::local::app::helpers_interface::transfer_from_seed(
        &seed,
        &destination,
        1_000_000,
        &blockhash,
        height,
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
    */

    // todo: shape
    // Ok(build_json_response(StatusCode::OK, text))
}
