mod openai;

use futures::StreamExt;
use openai::OpenAIWrapper;
use reqwest::Client;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "file",
    about = "A command line tool to help you find the right command for the job"
)]
struct CliArgs {
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
    let mut response_stream = Box::pin(
        open_ai_wrapper
            .get_streaming_response(&command_descripton)
            .await?,
    );

    while let Some(token) = response_stream.next().await {
        print!("{}", token);
    }

    Ok(())
}

fn get_api_key() -> anyhow::Result<String> {
    let key = std::env::var("OPENAI_API_KEY")?;
    Ok(key)
}
