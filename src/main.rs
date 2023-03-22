mod openai;

use anyhow::anyhow;
use futures::{Stream, StreamExt};
use openai::OpenAIWrapper;
use regex::Regex;
use reqwest::Client;
use std::pin::Pin;
use std::process::Command;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "file",
    about = "A command line tool to help you find the right command for the job"
)]
struct CliArgs {
    #[structopt(long, help = "Weather or not to stream the rosponse from OpenAI")]
    no_stream: bool,

    #[structopt(help = "Description of the command you want to find")]
    command_description: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = CliArgs::from_args();

    let client = Client::new();
    let openai_api_key = get_api_key()?;
    let open_ai_wrapper = OpenAIWrapper::new(&openai_api_key, &client);

    let command_descripton = opt.command_description.join(" ");

    let response: anyhow::Result<String> = if opt.no_stream {
        let response = open_ai_wrapper.get_response(&command_descripton).await?;
        println!("{}", response);
        Ok(response)
    } else {
        let response_stream = Box::pin(
            open_ai_wrapper
                .get_streaming_response(&command_descripton)
                .await?,
        );
        let response = print_and_extract_response(response_stream).await?;
        Ok(response)
    };

    let command = extract_code_block(&response?)?;
    println!("\nCommand to execute: {:?}", command);
    let result = execute_command(&command)?;
    print!("\nOutput:\n{}", result);

    Ok(())
}

async fn print_and_extract_response(
    mut stream: Pin<Box<impl Stream<Item = Result<String, anyhow::Error>>>>,
) -> anyhow::Result<String> {
    let mut response = String::new();
    while let Some(result_token) = stream.next().await {
        match result_token {
            Ok(token) => {
                print!("{}", token);
                response = format!("{}{}", response, token);
            }
            Err(e) => return Err(e),
        }
    }
    println!();
    Ok(response)
}

fn extract_code_block(input: &str) -> anyhow::Result<String> {
    let re = Regex::new(r"```(?:\w+)?\n?(?P<code>[\s\S]*?)\n?```")
        .map_err(|e| anyhow!("Error creating regex: {}", e))?;

    if let Some(captures) = re.captures(input) {
        let code = captures
            .name("code")
            .ok_or_else(|| anyhow!("No code block found"))?;
        Ok(code.as_str().to_string())
    } else {
        Err(anyhow!("No code block found"))
    }
}

fn execute_command(command: &str) -> anyhow::Result<String> {
    let output = Command::new("sh").arg("-c").arg(&command).output()?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        Err(anyhow::anyhow!(
            "Command failed: {}\nError message: {}",
            command,
            error_message
        ))
    } else {
        let success_message = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(success_message)
    }
}

fn get_api_key() -> anyhow::Result<String> {
    let key = std::env::var("OPENAI_API_KEY")?;
    Ok(key)
}
