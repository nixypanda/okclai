mod openai;

use futures::StreamExt;
use openai::OpenAIWrapper;
use regex::Regex;
use reqwest::Client;
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

    if opt.no_stream {
        let response = open_ai_wrapper.get_response(&command_descripton).await?;
        println!("{}", response);
    } else {
        let mut response_stream = Box::pin(
            open_ai_wrapper
                .get_streaming_response(&command_descripton)
                .await?,
        );

        let mut response = String::new();

        while let Some(result_token) = response_stream.next().await {
            match result_token {
                Ok(token) => {
                    print!("{}", token);
                    response = format!("{}{}", response, token);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    break;
                }
            }
        }
        println!();
        let command = extract_code_block(&response);
        if let Some(command) = command {
            println!("Command to execute: {:?}", command);
            let result = execute_command(&command)?;
            print!("Output:\n{}", result);
        }
    }

    Ok(())
}

fn extract_code_block(input: &str) -> Option<String> {
    let re = Regex::new(r"```(?:\w+)?\n?(?P<code>[\s\S]*?)\n?```").unwrap();
    if let Some(captures) = re.captures(input) {
        return Some(captures.name("code").unwrap().as_str().to_string());
    }
    None
}

fn execute_command(command: &str) -> anyhow::Result<String> {
    let output = Command::new("sh").arg("-c").arg(&command).output()?;
    // .with_context(|| format!("Failed to execute command: {}", command))?;

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
