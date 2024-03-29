# XMTP Message History Server

A simple, asynchronous file server to support XMTP Message History transfers. Running this server allows XMTP service providers to provide the ability for users to securely upload message history bundles via a `POST` request and retrieve them via a `GET` request using a unique ID assigned upon upload.   It is expected that these uploaded bundles should be: encrypted, short-lived, non-guessable, with scoped access to only authorized parties.

## Getting Started

### Prerequisites

Ensure you have the following installed:

- Rust and Cargo. You can install them from [https://rustup.rs/](https://rustup.rs/)

### Installing

Clone the repository to your local machine:

    git clone https://github.com/xmtp/xmtp-message-history-server.git
    cd xmtp-message-history-server

Build the project:

    cargo build

Run the server:

    cargo run

The server will start running on http://0.0.0.0:5558.

## Usage

### Uploading a File

To upload a file, send a POST request to http://0.0.0.1:5558/upload with the file data in the request body. The server will return a unique ID for the uploaded file.

Example using curl:

    curl -X POST -F "file=@path/to/your/file.txt" http://0.0.0.0:5558/upload

Retrieving a File

To retrieve an uploaded file, send a GET request to http://0.0.0.0:5558/files/{id}, where {id} is the unique ID returned by the server during the upload.

Example using curl:

    curl http://0.0.0.0:5558/files/{id} --output retrieved_file.txt

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.

## License

## This project is licensed under the MIT License - see the LICENSE file for details.
