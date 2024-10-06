use crate::error::AppError;
use crate::handlers::{
    AssistantEvent, AssistantStep, ChatInputEvent, ChatInputSkeletonEvent, ChatReplyEvent,
    ChatReplySkeletonEvent, SignalEvent, SpeechResult, COOKIE_NAME,
};
use crate::tools::{
    tool_completion_request, AnswerArgs, AssistantTool, DrawImageArgs, DrawImageResult,
    WriteCodeArgs, WriteCodeResult,
};
use crate::{audio_path, audio_url, image_path, image_url, EVENTS, LLM_SDK};
use anyhow::{anyhow, bail};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use comrak::markdown_to_html_with_plugins;
use comrak::plugins::syntect::SyntectAdapter;
use llm_sdk::{
    ChatCompleteModel, ChatCompletionChoice, ChatCompletionMessage, ChatCompletionRequest,
    CreateImageRequestBuilder, ImageResponseFormat, SpeechRequestBuilder, SpeechVoice,
    WhisperRequestBuilder, WhisperRequestType,
};
use salvo::http::form::FilePart;
use salvo::prelude::Text;
use salvo::{handler, Request, Response};
use serde_json::json;
use std::str::FromStr;
use tokio::fs;
use tokio::sync::broadcast;
use tracing::info;
use uuid::Uuid;

#[handler]
pub async fn assistant_handler(req: &mut Request, res: &mut Response) -> Result<(), AppError> {
    info!("Request id:{:?}", req.header::<String>("x-request-id"));
    info!("enter assistant handler");
    let device_id = req.cookie(COOKIE_NAME).unwrap().value().to_owned();
    let event_sender = EVENTS
        .get(&device_id)
        .ok_or_else(|| anyhow!("device_id not found for signal sender"))?
        .clone();
    info!("start assist for {}", device_id);

    let file = req
        .file("audio")
        .await
        .ok_or_else(|| AppError::from(anyhow!("No audio file")))?;

    match process(&event_sender, &device_id, file).await {
        Ok(_) => {
            res.render(Text::Json(json!({"status": "done"}).to_string()));
            Ok(())
        }
        Err(e) => {
            event_sender.send(error(e.to_string()))?;
            res.render(Text::Json(json!({"status": "error"}).to_string()));
            Ok(())
        }
    }
}

async fn process(
    event_sender: &broadcast::Sender<AssistantEvent>,
    device_id: &str,
    data: &FilePart,
) -> anyhow::Result<()> {
    let id = Uuid::new_v4().to_string();
    event_sender.send(in_audio_upload())?;

    info!("audio data size: {}", data.size());

    event_sender.send(in_transcription())?;
    event_sender.send(ChatInputSkeletonEvent::new(&id).into())?;

    let result = fs::read(data.path()).await?;
    let input = transcript(result).await?;
    event_sender.send(ChatInputEvent::new(&id, &input).into())?;

    event_sender.send(in_thinking())?;
    event_sender.send(ChatReplySkeletonEvent::new(&id).into())?;

    let choice = chat_completion_with_tools(&input).await?;

    match choice.finish_reason {
        llm_sdk::FinishReason::Stop => {
            let output = choice
                .message
                .content
                .ok_or_else(|| anyhow!("expect content but no content available"))?;
            event_sender.send(in_speech())?;
            let ret = SpeechResult::new_text_only(&output);
            event_sender.send(ChatReplyEvent::new(&id, ret).into())?;

            let ret = speech(device_id, &output).await?;
            event_sender.send(complete())?;
            event_sender.send(ChatReplyEvent::new(&id, ret).into())?;
        }
        llm_sdk::FinishReason::ToolCalls => {
            let tool_call = &choice.message.tool_calls[0].function;
            match AssistantTool::from_str(&tool_call.name) {
                Ok(AssistantTool::DrawImage) => {
                    let args: DrawImageArgs = serde_json::from_str(&tool_call.arguments)?;

                    event_sender.send(in_draw_image())?;
                    let ret = DrawImageResult::new("", &args.prompt);
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;

                    let ret = draw_image(device_id, args).await?;
                    event_sender.send(complete())?;
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;
                }
                Ok(AssistantTool::WriteCode) => {
                    event_sender.send(in_write_code())?;
                    let ret = write_code(serde_json::from_str(&tool_call.arguments)?).await?;
                    event_sender.send(complete())?;
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;
                }

                Ok(AssistantTool::Answer) => {
                    event_sender.send(in_chat_completion())?;
                    let output = answer(serde_json::from_str(&tool_call.arguments)?).await?;
                    event_sender.send(complete())?;
                    let ret = SpeechResult::new_text_only(&output);
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;

                    event_sender.send(in_speech())?;
                    let ret = speech(device_id, &output).await?;
                    event_sender.send(complete())?;
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;
                }
                _ => {
                    bail!("no proper tool found at the moment")
                }
            }
        }
        _ => {
            bail!("stop reason not supported")
        }
    }

    Ok(())
}

async fn transcript(data: Vec<u8>) -> anyhow::Result<String> {
    let req = WhisperRequestBuilder::default()
        .file(data)
        .prompt("If audio language is Chinese, please use Simplified Chinese")
        .request_type(WhisperRequestType::Transcription)
        .build()?;
    let res = LLM_SDK.whisper(req).await?;
    Ok(res.text)
}

async fn chat_completion_with_tools(prompt: &str) -> anyhow::Result<ChatCompletionChoice> {
    let req = tool_completion_request(prompt, "");
    let mut res = LLM_SDK.chat_completion(req).await?;
    let choice = res
        .choices
        .pop()
        .ok_or_else(|| anyhow!("expect at least one choice"))?;
    Ok(choice)
}

async fn chat_completion(messages: Vec<ChatCompletionMessage>) -> anyhow::Result<String> {
    let req = ChatCompletionRequest::new(ChatCompleteModel::default(), messages);
    let mut res = LLM_SDK.chat_completion(req).await?;
    let content = res
        .choices
        .pop()
        .ok_or_else(|| anyhow!("expect at least one choice"))?
        .message
        .content
        .ok_or_else(|| anyhow!("expect content but no content available"))?;
    Ok(content)
}

async fn speech(device_id: &str, text: &str) -> anyhow::Result<SpeechResult> {
    let req = SpeechRequestBuilder::default()
        .input(text)
        // switch voice
        .voice(SpeechVoice::Alloy)
        .build()?;
    let data = LLM_SDK.speech(req).await?;
    let uuid = Uuid::new_v4().to_string();
    let path = audio_path(device_id, &uuid);
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    tokio::fs::write(&path, data).await?;
    Ok(SpeechResult::new(text, audio_url(device_id, &uuid)))
}

async fn draw_image(device_id: &str, args: DrawImageArgs) -> anyhow::Result<DrawImageResult> {
    let req = CreateImageRequestBuilder::default()
        .prompt(args.prompt)
        .response_format(ImageResponseFormat::B64Json)
        .build()?;
    let mut ret = LLM_SDK.create_image(req).await?;
    let img = ret
        .data
        .pop()
        .ok_or_else(|| anyhow!("expect at least one data"))?;
    let data = BASE64_STANDARD.decode(img.b64_json.unwrap())?;
    let uuid = Uuid::new_v4().to_string();
    let path = image_path(device_id, &uuid);
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await?;
        }
    }
    fs::write(&path, data).await?;
    Ok(DrawImageResult::new(
        image_url(device_id, &uuid),
        img.revised_prompt,
    ))
}

async fn write_code(args: WriteCodeArgs) -> anyhow::Result<WriteCodeResult> {
    let messages = vec![
        ChatCompletionMessage::new_system("I'm an expert on coding, I'll write code for you in markdown format based on your prompt", "Ava"),
        ChatCompletionMessage::new_user(args.prompt, ""),

    ];
    let md = chat_completion(messages).await?;
    Ok(WriteCodeResult::new(md2html(&md)))
}

async fn answer(args: AnswerArgs) -> anyhow::Result<String> {
    let messages = vec![
        ChatCompletionMessage::new_system("I can help answer anything you'd like to chat", "Ava"),
        ChatCompletionMessage::new_user(args.prompt, ""),
    ];
    chat_completion(messages).await
}

fn md2html(md: &str) -> String {
    let adapter = SyntectAdapter::new(Some("Solarized (dark)"));
    let options = comrak::Options::default();
    let mut plugins = comrak::Plugins::default();

    plugins.render.codefence_syntax_highlighter = Some(&adapter);
    markdown_to_html_with_plugins(md, &options, &plugins)
}

fn in_audio_upload() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::UploadAudio).into()
}

fn in_transcription() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::Transcription).into()
}

fn in_thinking() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::Thinking).into()
}

fn in_chat_completion() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::ChatCompletion).into()
}

fn in_speech() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::Speech).into()
}

fn in_draw_image() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::DrawImage).into()
}

fn in_write_code() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::WriteCode).into()
}

fn complete() -> AssistantEvent {
    SignalEvent::Complete.into()
}

fn error(msg: impl Into<String>) -> AssistantEvent {
    SignalEvent::Error(msg.into()).into()
}
