use reqwest::blocking::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const PROMPT_PREFIX: &str = "Assume you are a Linux/Unix expert. \
    Be concise in your response. \
    Which command ";

#[derive(Debug, StructOpt)]
#[structopt(
    name = "file",
    about = "A command line tool to help you find the right command for the job"
)]
struct CliArgs {
    #[structopt(help = "Description of the command you want to find")]
    command_description: String,
}

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

struct OpenAIWrapper<'a> {
    model: &'a str,
    api_endpoint: &'a str,
    api_key: &'a str,
    prompt_prefix: &'a str,
    client: &'a Client,
}

impl<'a> OpenAIWrapper<'a> {
    fn new(api_key: &'a str, client: &'a Client) -> Self {
        OpenAIWrapper {
            model: "gpt-3.5-turbo",
            api_endpoint: OPENAI_API_URL,
            api_key: api_key,
            prompt_prefix: PROMPT_PREFIX,
            client: &client,
        }
    }

    fn make_request(&self, command_description: &str) -> anyhow::Result<RequestBuilder> {
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

    fn get_response(&self, operation: &str) -> anyhow::Result<String> {
        let request = self.make_request(operation)?;
        let response = request.send()?;

        if response.status() != reqwest::StatusCode::OK {
            return Err(anyhow::anyhow!(
                "Response code is not 200 OK: {:?}",
                response.status()
            ));
        }

        let response_body: GPTResponse = response.json()?;
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

fn main() -> anyhow::Result<()> {
    let opt = CliArgs::from_args();

    let client = Client::new();
    let openai_api_key = get_api_key()?;
    let open_ai_wrapper = OpenAIWrapper::new(&openai_api_key, &client);

    let response = open_ai_wrapper.get_response(&opt.command_description)?;
    println!("{response}");

    Ok(())
}

fn get_api_key() -> anyhow::Result<String> {
    let key = std::env::var("OPENAI_API_KEY")?;
    Ok(key)
}
