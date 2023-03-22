# Ok ClAI

This command-line tool built in Rust enables you to find and execute the right
command by entering a description of the desired functionality. It accomplishes
this by leveraging OpenAI's API.

Note: The name is a derived from "Ok Google" + "CLI" + "AI"

## Prerequisites

- Rust programming language
- OpenAI API key

## Installation

This is just a fun weekend project, so there are packages provided for various
operating systems. Currently, you have to just clone the repository and then run
it.

1. Clone the repository and navigate to the project directory

   `$ git clone https://github.com/sherub/okclai.git`

   `$ cd okclai>`

2. Set the `OPENAI_API_KEY` environment variable to your OpenAI API key

   `$ export OPENAI_API_KEY=<your_api_key>`

3. Build the executable

   `$ cargo build --release`

## Usage

The tool takes in a description of the desired functionality as an argument and
returns the corresponding command. You can choose to stream the response or get
a single response.

```shell
A command line tool to help you find and execute the right command form a description

USAGE:
    okclai [FLAGS] [command-description]...

FLAGS:
    -h, --help              Prints help information
        --no-ask            Weather or not to ask before executing the command
        --no-explanation    Weather or not to explain what is going on
        --no-stream         Weather or not to stream the rosponse from OpenAI
    -V, --version           Prints version information

ARGS:
    <command-description>...    Description of the command you want to find
```

Here is an example command that streams the response, and does not ask before
executing the command:

`$ ./target/release/okclai --no-ask show me all the git remote url for this repository`

Here is an example command that returns a single response, and does not show the
response, and does not ask before executing the command:

```shell
./target/release/okclai \
    --no-stream \
    --no-ask \
    --no-explanation \
    create a new folder named my_folder`
```

## License

This project is licensed under the MIT License - do whatever you want with it.
