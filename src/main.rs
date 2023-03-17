use serde::{Deserialize, Serialize};
use serde_json::Value;
use structopt::StructOpt;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Serialize)]
struct GPTRequest<'a> {
    model: &'a str,
    messages: &'a [Chat<'a>],
}

#[derive(Serialize)]
struct Chat<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "file",
    about = "A command line tool to help you find the right command for the job"
)]
struct Opt {
    #[structopt(help = "The operation you want to perform")]
    operation: String,
}

fn get_response(operation: &str) -> anyhow::Result<String> {
    let prompt = format!("What is a command that {}?", operation);
    let message = Chat {
        role: "user",
        content: &prompt,
    };
    let request_body = GPTRequest {
        model: "gpt-3.5-turbo",
        messages: &[message],
    };
    let client = reqwest::blocking::Client::new();
    let request = client
        .post(OPENAI_API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", get_api_key()))
        .json(&request_body);
    let response = request.send()?;
    let response_body: Value = response.json()?;
    let response_content = response_body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap()
        .to_string();

    Ok(response_content)
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    let response = get_response(&opt.operation)?;
    println!("{response}");

    Ok(())
}

fn get_api_key() -> String {
    // TODO: Read from environment variable "OPENAI_API_KEY"
    std::env::var("OPENAI_API_KEY").unwrap()
}
