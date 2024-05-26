# Rust AI Stream Analyzer Twitch Bot

Rust-ai-streambot is a transformative AI pipeline completely built in Rust. Focused on Transformer/Tensor code and taking advantage of the Candle framework by Huggingface, this systems programming language approach provides a robust interface to AI model interaction and stream analysis. It enables local execution on MacOS devices equipped with M1/M2/M3 ARM GPUs, bypassing the need for external dependencies and Python code and facilitating the integration of local large language models (LLMs) with Rust.

## Key Features

-   **Local LLM**: Leverages Rust-based LLMs, Mistral and Gemma, from the Candle framework for direct and efficient AI operations prioritizing local execution to harness the full power of MacOS Metal GPUs.
-   **Comprehensive AI Analyzer**: Incorporates a high-level AI analyzer capable of processing and generating across various domains, facilitating a continuous flow of AI-generated content (In Progress).
-   **Voice and Speech Integration**: Plans to integrate with Whisper for voice-driven interactions, allowing users to interact with the toolkit using voice commands and receive streaming text in return (Planned Feature).
-   **Image Generation and NDI Output**: Supports generating images from text descriptions and outputting via NDI for a wide range of applications, including real-time content creation and broadcasting (In Beta Testing).
-   **Twitch Chat Interactive AI**: Integrated Twitch chat for real-time AI interactions, enabling users to engage with the toolkit through chat commands and receive AI-generated responses.

and many more...

## Installing Rust AI Stream Analyzer Twitch Bot

-   Ensure Rust and Cargo are installed. [Rust Installation Guide](https://www.rust-lang.org/tools/install).
-   A MacOS system with an M1/M2/M3 ARM GPU is ideal.

1. **Clone the Repository**:

    ```bash
    git clone https://github.com/hongyeecheah/rust-ai-streambot.git
    ```

2. **Navigate to the Project Directory**:

    ```bash
    cd rust-ai-streambot
    ```

3. **Compile with Metal GPU Support and NDI SDK support**:

    ```bash
    ./scripts/compile.sh
    ```

## Usage

Rust AI Stream Analyzer Twitch Bot offers a broad range of AI-driven functionalities, including generating text-based content, analyzing network streams, and processing visual and audio inputs. More advanced features are in development. Follow the example commands in the documentation to get started.

## License

This project is licensed under the MIT License.

## Author

Hongyee Cheah, dedicated to the development of pioneering AI solutions with the Rust AI Stream Analyzer Twitch Bot. February 2024.