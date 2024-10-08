mod assistant;
mod chats;
mod common;

use askama::Template;
pub use assistant::*;
pub use chats::*;
pub use common::*;
use derive_more::From;
use std::fmt::Debug;

use crate::tools::{DrawImageResult, WriteCodeResult};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use time::macros::{format_description, offset};
use time::OffsetDateTime;

#[derive(Debug, Clone, From)]
pub(crate) enum AssistantEvent {
    Signal(SignalEvent),
    InputSkeleton(ChatInputSkeletonEvent),
    Input(ChatInputEvent),
    ReplySkeleton(ChatReplySkeletonEvent),
    Reply(ChatReplyEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "events/signal.html.j2")]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub(crate) enum SignalEvent {
    Processing(AssistantStep),
    Error(String),
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "events/chat_input_skeleton.html.j2")]
pub(crate) struct ChatInputSkeletonEvent {
    id: String,
    datetime: String,
    avatar: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "events/chat_input.html.j2")]
pub(crate) struct ChatInputEvent {
    id: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "events/chat_reply_skeleton.html.j2")]
pub(crate) struct ChatReplySkeletonEvent {
    id: String,
    avatar: String, // /public/images/ava-small.png
    name: String,   // Ava
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "events/chat_reply.html.j2")]
pub(crate) struct ChatReplyEvent {
    id: String,
    data: ChatReplyData,
}

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ChatReplyData {
    Speech(SpeechResult),
    Image(DrawImageResult),
    Markdown(WriteCodeResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "blocks/speech.html.j2")]
pub(crate) struct SpeechResult {
    text: String,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub(crate) enum AssistantStep {
    #[strum(serialize = "Uploading audio")]
    UploadAudio,
    #[strum(serialize = "Transcribing audio")]
    Transcription,
    #[strum(serialize = "Thinking hard")]
    Thinking,
    #[strum(serialize = "Organizing answer")]
    ChatCompletion,
    #[strum(serialize = "Drawing image")]
    DrawImage,
    #[strum(serialize = "Writing code")]
    WriteCode,
    #[strum(serialize = "Generating speech")]
    Speech,
}

impl ChatInputSkeletonEvent {
    pub fn new(id: impl Into<String>) -> Self {
        let datetime = OffsetDateTime::now_utc()
            .to_offset(offset!(+08:00:00))
            .format(format_description!(
                "[year]-[month]-[day] [hour]:[minute]:[second]"
            ))
            .unwrap();
        Self {
            id: id.into(),
            datetime,
            avatar: "https://i.pravatar.cc/128".to_string(),
            name: "User".to_string(),
        }
    }
}

impl ChatInputEvent {
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
        }
    }
}

impl ChatReplySkeletonEvent {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            avatar: "./public/images/ava-small.png".to_string(),
            name: "Ava".to_string(),
        }
    }
}

impl ChatReplyEvent {
    pub fn new(id: impl Into<String>, data: impl Into<ChatReplyData>) -> Self {
        Self {
            id: id.into(),
            data: data.into(),
        }
    }
}

impl SpeechResult {
    fn new(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
        }
    }

    fn new_text_only(text: impl Into<String>) -> Self {
        Self::new(text, "".to_string())
    }
}

impl From<AssistantEvent> for String {
    fn from(event: AssistantEvent) -> Self {
        match event {
            AssistantEvent::Signal(v) => v.into(),
            AssistantEvent::InputSkeleton(v) => v.into(),
            AssistantEvent::Input(v) => v.into(),
            AssistantEvent::ReplySkeleton(v) => v.into(),
            AssistantEvent::Reply(v) => v.into(),
        }
    }
}

impl From<SignalEvent> for String {
    fn from(event: SignalEvent) -> Self {
        event.render().unwrap()
    }
}

impl From<ChatInputSkeletonEvent> for String {
    fn from(event: ChatInputSkeletonEvent) -> Self {
        event.render().unwrap()
    }
}

impl From<ChatInputEvent> for String {
    fn from(event: ChatInputEvent) -> Self {
        event.render().unwrap()
    }
}

impl From<ChatReplySkeletonEvent> for String {
    fn from(event: ChatReplySkeletonEvent) -> Self {
        event.render().unwrap()
    }
}

impl From<ChatReplyEvent> for String {
    fn from(event: ChatReplyEvent) -> Self {
        event.render().unwrap()
    }
}
