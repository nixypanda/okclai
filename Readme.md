# Which Command

This command-line tool built in Rust enables you to find the right command by
entering a description of the desired functionality. It accomplishes this by
leveraging OpenAI's API.

## Prerequisites

- Rust programming language
- OpenAI API key

## Installation

This is just a fun weekend project, so there are packages provided for various
operating systems. Currently, you have to just clone the repository and then run
it.

1. Clone the repository and navigate to the project directory

   `$ git clone https://github.com/sherub/which_command.git`

   `$ cd which_command>`

2. Set the `OPENAI_API_KEY` environment variable to your OpenAI API key

   `$ export OPENAI_API_KEY=<your_api_key>`

3. Build the executable

   `$ cargo build --release`

## Usage

The tool takes in a description of the desired functionality as an argument and
returns the corresponding command. You can choose to stream the response or get
a single response.

```shell
A command line tool to help you find the right command for the job

USAGE:
    which_command [FLAGS] [command-description]...

FLAGS:
    -h, --help         Prints help information
        --no-stream    Weather or not to stream the rosponse from OpenAI
    -V, --version      Prints version information

ARGS:
    <command-description>...    Description of the command you want to find
```

Here is an example command that streams the response:

`$ ./target/release/which_command Move a file from one directory to another`

Here is an example command that returns a single response:

`$ ./target/release/which_command --no-stream Create a new folder in Windows`

## License

This project is licensed under the MIT License - do whatever you want with it.
