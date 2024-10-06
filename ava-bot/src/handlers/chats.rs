use crate::handlers::{AssistantEvent, COOKIE_NAME};
use crate::EVENTS;
use dashmap::DashMap;
use salvo::prelude::SseKeepAlive;
use salvo::sse::SseEvent;
use salvo::{handler, Request, Response};
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt as _};
use tracing::info;

const MAX_EVENTS: usize = 128;
#[handler]
pub async fn events_handler(req: &mut Request, res: &mut Response) {
    let device_id = req.cookie(COOKIE_NAME).unwrap().value().to_owned();

    info!("user {device_id} connected");
    sse_handler(&device_id, &EVENTS, res).await;
}

async fn sse_handler(
    device_id: &str,
    map: &DashMap<String, broadcast::Sender<AssistantEvent>>,
    res: &mut Response,
) {
    let rx = if let Some(tx) = map.get(device_id) {
        tx.subscribe()
    } else {
        let (tx, rx) = broadcast::channel(MAX_EVENTS);
        map.insert(device_id.to_string(), tx);
        rx
    };

    let stream = BroadcastStream::new(rx)
        .filter_map(|v| v.ok())
        .map(|v| {
            let (event, id) = match &v {
                AssistantEvent::Signal(_) => ("signal", "".to_string()),
                AssistantEvent::InputSkeleton(_) => ("input_skeleton", "".to_string()),
                AssistantEvent::Input(v) => ("input", v.id.clone()),
                AssistantEvent::ReplySkeleton(_) => ("reply_skeleton", "".to_string()),
                AssistantEvent::Reply(v) => ("reply", v.id.clone()),
            };
            let data: String = v.into();
            SseEvent::default().name(event).text(data).id(id)
        })
        .map(Ok::<_, Infallible>);
    SseKeepAlive::new(stream)
        .comment("keep-alive-text")
        .max_interval(Duration::from_secs(1))
        .stream(res);
}
