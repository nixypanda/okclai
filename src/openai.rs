use eventsource_client::{
    Client as EventSourceClient, ClientBuilder as EventSourceClientBuilder, SSE,
};
use futures_util::TryStreamExt;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::pin::Pin;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const PROMPT_PREFIX: &str = "Assume you are a Linux/Unix expert. \
    Be concise in your response. \
    Which command ";

#[derive(Debug, Serialize, Deserialize)]
struct ChatFormatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct GPTRequest<'a> {
    model: &'a str,
    messages: &'a [&'a ChatFormatMessage],
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct GPTResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    message: ChatFormatMessage,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum StreamingChatFormatMessage {
    Role { role: String },
    Content { content: String },
    Empty {},
}

#[derive(Debug, Deserialize)]
struct StreamingGPTResponse {
    choices: Vec<StreamingChoice>,
}

#[derive(Debug, Deserialize, Clone)]
struct StreamingChoice {
    delta: StreamingChatFormatMessage,
    finish_reason: Option<String>,
}

pub struct OpenAIWrapper<'a> {
    model: &'a str,
    api_endpoint: &'a str,
    api_key: &'a str,
    prompt_prefix: &'a str,
    client: &'a Client,
}

impl<'a> OpenAIWrapper<'a> {
    pub fn new(api_key: &'a str, client: &'a Client) -> Self {
        OpenAIWrapper {
            model: "gpt-3.5-turbo",
            api_endpoint: OPENAI_API_URL,
            api_key: api_key,
            prompt_prefix: PROMPT_PREFIX,
            client: &client,
        }
    }

    async fn make_request(&self, command_description: &str) -> anyhow::Result<RequestBuilder> {
        let prompt = format!("{} {}?", self.prompt_prefix, command_description);
        let message = ChatFormatMessage {
            role: "user".to_string(),
            content: prompt,
        };
        let request_body = GPTRequest {
            model: &self.model,
            messages: &[&message],
            stream: false,
        };

        let request = self
            .client
            .post(self.api_endpoint)
            .header("Content-Type", "application/json")
            .header("Authorization", &format!("Bearer {}", &self.api_key))
            .json(&request_body);

        Ok(request)
    }

    pub async fn get_response(&self, operation: &str) -> anyhow::Result<String> {
        let request = self.make_request(operation).await?;
        let response = request.send().await?;

        if response.status() != reqwest::StatusCode::OK {
            return Err(anyhow::anyhow!(
                "Response code is not 200 OK: {:?}",
                response.status()
            ));
        }

        let response_body: GPTResponse = response.json().await?;
        let response_content = match response_body.choices.first() {
            Some(choice) => choice.message.content.to_string(),
            None => {
                return Err(anyhow::anyhow!(
                    "No choices in response body: {:?}",
                    response_body
                ))
            }
        };

        Ok(response_content)
    }

    pub async fn get_streaming_response(
        &self,
        command_description: &str,
    ) -> anyhow::Result<StreamingResponseIterator> {
        let prompt = format!("{} {}?", self.prompt_prefix, command_description);
        let message = ChatFormatMessage {
            role: "user".to_string(),
            content: prompt,
        };
        let gpt_request = GPTRequest {
            model: &self.model,
            messages: &[&message],
            stream: true,
        };

        let request_body = serde_json::to_string(&gpt_request).unwrap();
        let request = EventSourceClientBuilder::for_url(self.api_endpoint)
            .unwrap()
            .header("Authorization", &format!("Bearer {}", &self.api_key))
            .unwrap()
            .header("Content-Type", "application/json")
            .unwrap()
            .method("POST".to_string())
            .body(request_body)
            .build();

        let stream = Box::pin(request.stream());

        Ok(StreamingResponseIterator::new(stream))

        // while let Ok(Some(event)) = stream.try_next().await {
        //     match event {
        //         SSE::Comment(comment) => println!("got a comment event: {:?}", comment),
        //         SSE::Event(evt) => {
        //             let data: StreamingGPTResponse = serde_json::from_str(&evt.data).unwrap();
        //             let choice = data.choices[0].clone();
        //             let delta = choice.delta.clone();
        //             println!("got an event: {:?}", delta);
        //             if let StreamingChatFormatMessage::Content { content } = delta {
        //                 response = format!("{}{}", response, content);
        //             } else if let StreamingChatFormatMessage::Empty {} = delta {
        //                 break;
        //             }
        //         }
        //     }
        // }
        //
        // Ok("dea".to_string())
    }
}

pub struct StreamingResponseIterator {
    stream:
        Pin<Box<dyn futures_util::stream::Stream<Item = Result<SSE, eventsource_client::Error>>>>,
}

impl StreamingResponseIterator {
    pub fn new(
        stream: Pin<
            Box<dyn futures_util::stream::Stream<Item = Result<SSE, eventsource_client::Error>>>,
        >,
    ) -> StreamingResponseIterator {
        StreamingResponseIterator { stream }
    }
}

impl Iterator for StreamingResponseIterator {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        match futures::executor::block_on(self.stream.try_next()) {
            Ok(Some(event)) => match event {
                SSE::Comment(comment) => {
                    // println!("got a comment event: {:?}", comment);
                    panic!("TODO");
                }
                SSE::Event(evt) => {
                    let data: StreamingGPTResponse = serde_json::from_str(&evt.data).unwrap();
                    let choice = data.choices[0].clone();
                    let delta = choice.delta.clone();
                    // println!("got an event: {:?}", delta);
                    match delta {
                        StreamingChatFormatMessage::Content { content } => Some(content),
                        StreamingChatFormatMessage::Empty {} => None,
                        StreamingChatFormatMessage::Role { role } => Some("BEGIN:".to_string()),
                    }
                }
            },
            Ok(None) => None,
            Err(e) => None,
        }
    }
}
