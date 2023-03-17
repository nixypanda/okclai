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
}

#[derive(Debug, Deserialize)]
struct GPTResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    message: ChatFormatMessage,
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
}
