//! A local OpenAI-compatible endpoint for real-binary transport checks.
use std::{
    collections::VecDeque,
    convert::Infallible,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use http_body_util::{BodyExt, Full, Limited};
use hyper::{Request, Response, body::Bytes, service::service_fn};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

pub struct ProviderFixture {
    endpoint: String,
    cancel: CancellationToken,
    server: Option<JoinHandle<()>>,
}

impl ProviderFixture {
    pub fn new(responses: Vec<Value>) -> Self {
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
        let cancel = CancellationToken::new();
        let stopped = cancel.clone();
        let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
        let server = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                    loop {
                        let stream = tokio::select! {
                            () = stopped.cancelled() => break,
                            result = listener.accept() => result.unwrap().0,
                        };
                        let responses = responses.clone();
                        tokio::spawn(async move {
                            let service =
                                service_fn(move |request| reply(request, responses.clone()));
                            let _ = hyper::server::conn::http1::Builder::new()
                                .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                                .await;
                        });
                    }
                });
        });
        Self {
            endpoint,
            cancel,
            server: Some(server),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl Drop for ProviderFixture {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(server) = self.server.take() {
            server.join().unwrap();
        }
    }
}

async fn reply(
    request: Request<hyper::body::Incoming>,
    responses: Arc<Mutex<VecDeque<Value>>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    if Limited::new(request.into_body(), 64 * 1024)
        .collect()
        .await
        .is_err()
    {
        return Ok(Response::builder()
            .status(413)
            .body(Full::new(Bytes::new()))
            .unwrap());
    }
    let response = responses
        .lock()
        .unwrap()
        .pop_front()
        .unwrap_or_else(|| json!({"content":"Fixture complete"}));
    let mut delta = json!({"content":response["content"]});
    if let Some(calls) = response["tool_calls"].as_array() {
        delta["tool_calls"] = json!(
            calls
                .iter()
                .enumerate()
                .map(|(index, call)| json!({
                    "index":index,"id":call["id"],"type":"function", "function":{
                        "name":call["name"],"arguments":call["arguments"].to_string(),
                    },
                }))
                .collect::<Vec<_>>()
        );
    }
    let body = format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"delta":delta}]})
    );
    Ok(Response::builder()
        .header("content-type", "text/event-stream")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}
