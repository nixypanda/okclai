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

#[derive(Serialize)]
struct GPTReq<'a> {
    model: &'a str,
    messages: &'a [&'a ChatFmtMsg],
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct GPTRes {
    choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    #[serde(alias = "delta")]
    message: ChatFmtMsg,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
enum ChatFmtMsg {
    Role { role: String },
    Content { content: String },
    Both { role: String, content: String },
    Empty {},
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
        let message = ChatFmtMsg::Both {
            role: "user".to_string(),
            content: prompt,
        };
        let request_body = GPTReq {
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

        let response_body: GPTRes = response.json().await?;
        let response_content = match response_body.choices.first() {
            Some(choice) => {
                if let ChatFmtMsg::Both { role: _, content } = choice.message.clone() {
                    Ok(content)
                } else {
                    Err(anyhow::anyhow!("Unexpected response content",))
                }
            }
            None => Err(anyhow::anyhow!(
                "No choices in response body: {:?}",
                response_body
            )),
        }?;

        Ok(response_content)
    }

    fn make_streaming_request(
        &self,
        command_description: &str,
    ) -> Result<impl eventsource_client::Client, eventsource_client::Error> {
        let prompt = format!("{} {}?", self.prompt_prefix, command_description);
        let message = ChatFmtMsg::Both {
            role: "user".to_string(),
            content: prompt,
        };
        let gpt_request = GPTReq {
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
                    Ok(SSE::Event(evt)) => Some(parse_event(&evt.data)),
                    Ok(SSE::Comment(_)) => None,
                    Err(e) => Some(Err(anyhow::anyhow!("Error receiving stream: {:?}", e))),
                }
            })
            .map(|rr| rr.map_or_else(Err, get_first_choice))
            .skip_while(|rd| future::ready(matches!(&rd, Ok(ChatFmtMsg::Role { role: _ }))))
            .take_while(|rd| future::ready(!matches!(&rd, Ok(ChatFmtMsg::Empty {}))))
            .map(|rd| match rd {
                Ok(ChatFmtMsg::Content { content }) => Ok(content),
                Ok(chat_fmt) => Err(anyhow::anyhow!("Unexpected content: {:?}", chat_fmt)),
                Err(e) => Err(e.into()),
            });

        Ok(response)
    }
}

fn parse_event(event: &str) -> anyhow::Result<GPTRes> {
    let result = serde_json::from_str::<GPTRes>(event);
    match result {
        Ok(json) => Ok(json),
        Err(e) => Err(anyhow::anyhow!("Error parsing streaming response: {:?}", e)),
    }
}

fn get_first_choice(res: GPTRes) -> anyhow::Result<ChatFmtMsg> {
    res.choices
        .first()
        .map(|choice| choice.message.clone())
        .ok_or_else(|| anyhow::anyhow!("Expected one choice"))
}
