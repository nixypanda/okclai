use eventsource_client::{
    Client as EventSourceClient, ClientBuilder as EventSourceClientBuilder, SSE,
};
use futures::{future, Stream, StreamExt};
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize, Clone, PartialEq)]
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
pub struct StreamingChoice {
    delta: StreamingChatFormatMessage,
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
            api_key,
            prompt_prefix: PROMPT_PREFIX,
            client,
        }
    }

    async fn make_request(&self, command_description: &str) -> anyhow::Result<RequestBuilder> {
        let prompt = format!("{} {}?", self.prompt_prefix, command_description);
        let message = ChatFormatMessage {
            role: "user".to_string(),
            content: prompt,
        };
        let request_body = GPTRequest {
            model: self.model,
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

    fn make_streaming_request(
        &self,
        command_description: &str,
    ) -> Result<impl eventsource_client::Client, eventsource_client::Error> {
        let prompt = format!("{} {}?", self.prompt_prefix, command_description);
        let message = ChatFormatMessage {
            role: "user".to_string(),
            content: prompt,
        };
        let gpt_request = GPTRequest {
            model: self.model,
            messages: &[&message],
            stream: true,
        };
        let request_body = serde_json::to_string(&gpt_request)?;
        let request = EventSourceClientBuilder::for_url(self.api_endpoint)?
            .header("Authorization", &format!("Bearer {}", &self.api_key))?
            .header("Content-Type", "application/json")?
            .method("POST".to_string())
            .body(request_body)
            .build();

        Ok(request)
    }

    pub async fn get_streaming_response(
        &self,
        command_description: &str,
    ) -> anyhow::Result<impl Stream<Item = anyhow::Result<String>>> {
        let request = self
            .make_streaming_request(command_description)
            .map_err(|err| anyhow::anyhow!("Error making streaming request: {:?}", err))?;

        let stream = Box::pin(request.stream());

        let response = stream
            .filter_map(|event| async move {
                match event {
                    Ok(SSE::Event(evt)) => {
                        let result_json = serde_json::from_str::<StreamingGPTResponse>(&evt.data);
                        match result_json {
                            Ok(json) => Some(Ok(json)),
                            Err(e) => Some(Err(anyhow::anyhow!(
                                "Error parsing streaming response: {:?}",
                                e
                            ))),
                        }
                    }
                    Ok(SSE::Comment(comment)) => {
                        // log this shit out somewhere
                        None
                    }
                    Err(e) => Some(Err(anyhow::anyhow!("Error receiving stream: {:?}", e))),
                }
            })
            .map(|result_response| {
                result_response.map(|response| response.choices[0].delta.clone())
            })
            .skip_while(|result_delta| {
                future::ready(!matches!(
                    &result_delta,
                    Ok(StreamingChatFormatMessage::Role { role: _ })
                ))
            })
            .take_while(|result_delta| {
                future::ready(!matches!(
                    &result_delta,
                    Ok(StreamingChatFormatMessage::Empty {})
                ))
            })
            .filter_map(|result_delta| async move {
                match result_delta {
                    Ok(StreamingChatFormatMessage::Content { content }) => Some(Ok(content)),
                    _ => None,
                }
            });

        Ok(response)
    }
}
