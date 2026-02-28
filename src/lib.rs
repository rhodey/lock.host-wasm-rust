use form_urlencoded;
use wstd::http::body::IncomingBody;
use wstd::http::Client;
use wstd::http::server::{Finished, Responder};
use wstd::http::{IntoBody, Method, Request, Response, StatusCode};
use wstd::io::{copy, AsyncWrite};
use wstd::time::{Duration, Instant};

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

async fn send_json(url: &str, bearer: Option<&str>, payload: String) -> Result<String, String> {
    let mut req = Request::builder()
        .method(Method::POST)
        .uri(url)
        .header("content-type", "application/json")
        .body(payload.into_body())
        .map_err(|e| e.to_string())?;

    if let Some(api_key) = bearer {
        req.headers_mut()
            .insert("authorization", format!("Bearer {api_key}").parse().unwrap());
    }

    let mut resp = Client::new().send(req).await.map_err(|e| e.to_string())?;
    let body = String::from_utf8(resp.body_mut().bytes().await.map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(body)
}

async fn get_balance(req: Request<IncomingBody>, responder: Responder) -> Finished {
    let Some(address) = query_param(&req, "addr") else {
        return bad_request(responder, "missing query param `addr`\n").await;
    };

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [address]
    })
    .to_string();

    let output = match send_json("https://api.devnet.solana.com", None, payload).await {
        Ok(body) => body,
        Err(err) => format!("{{\"error\":\"{err}\"}}"),
    };

    let response = Response::builder()
        .status(StatusCode::OK)
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

    let output = match send_json(
        "https://api.openai.com/v1/chat/completions",
        Some(&api_key),
        payload,
    )
    .await
    {
        Ok(body) => body,
        Err(err) => format!("{{\"error\":\"{err}\"}}"),
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(output.into_body())
        .unwrap();
    responder.respond(response).await
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
    let now = Instant::now();
    wstd::task::sleep(Duration::from_secs(1)).await;
    let elapsed = Instant::now().duration_since(now).as_millis();

    let mut body = res.start_response(Response::new(wstd::http::body::BodyForthcoming));
    let result = body
        .write_all(format!("slept for {elapsed} millis\n").as_bytes())
        .await;
    Finished::finish(body, result, None)
}

async fn echo(mut req: Request<IncomingBody>, res: Responder) -> Finished {
    let mut body = res.start_response(Response::new(wstd::http::body::BodyForthcoming));
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
