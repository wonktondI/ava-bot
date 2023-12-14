use std::convert::Infallible;
use futures_util::StreamExt;
use salvo::sse::SseEvent;
use salvo::{handler, sse, Response};
use std::time::Duration;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;

#[handler]
pub async fn chats_handler(res: &mut Response) {
    let event_stream = {
        let interval = interval(Duration::from_secs(1));
        let stream = IntervalStream::new(interval);
        stream.map(move |_| Ok::<SseEvent, Infallible>(SseEvent::default().text("<li>hello world!</li>".to_string())))
    };
    sse::stream(res, event_stream);
}
