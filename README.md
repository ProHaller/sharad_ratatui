# Sharad Ratatui

Welcome to Sharad Ratatui, a text-based Shadowrun role-playing game built with Rust and the Ratatui library!

<https://github.com/user-attachments/assets/54ca4490-0df8-4e6d-a6fb-7d61336547f5>

## 🌟 Join Our Adventure

Are you passionate about gaming and development? Do you want to be part of a project that combines the excitement of RPGs with the creativity of programming? Then you've come to the right place!

As a complete programming beginner, I've started this journey to create Sharad Ratatui, and I'm thrilled to invite you to join me. Whether you're a seasoned developer or just starting out, your contributions are not just welcome – they're essential to our growth!

### About the Game

Sharad Ratatui is a text-based RPG set in the world of Shadowrun, an iconic tabletop RPG known for its blend of cyberpunk and fantasy elements. Traditionally, a game master (GM) guides players through the narrative, responding to their decisions and shaping the world around them.

In Sharad Ratatui, the role of the GM is handled by a mix of AI for the Narration and the game's logic for dice based rules application. Players input natural text commands to decide their actions, and the game generates the narrative, dialogues, and other story elements in real-time. The game keeps track of all the variables, updating the character sheet and world state as the player progresses through their adventure. This approach offers a unique and dynamic storytelling experience, making each playthrough distinct. Furthermore, the narration can be read by the game with multiple voices and styles, allowing for more immersive and engaging gameplay.

### Why Contribute?

- **Learn and Grow**: Dive into Rust development and explore the world of terminal user interfaces.
- **Beginner-Friendly**: As a beginner myself, I definitely won't judge, and although I've tried to structure this project to be accessible to newcomers, it's a bit of a mess right now.
- **Innovative Gameplay**: Help shape a unique AIRPG experience.
- **Rust & Ratatui**: Gain experience with Rust and terminal user interfaces.

### Requirements

To play Sharad Ratatui, you will need an OpenAI API Key. This key is necessary to enable the game's narrative generation and other dynamic features. If you don't have an API key but still want to contribute, you can contact the maintainer to get sponsored.

You can provide your API key in two ways:

1. **Environment Variable** (recommended): Set the `OPENAI_API_KEY` environment variable
2. **In-game Settings**: Enter your API key through the game's settings menu

## 🛠️ Project Architecture Overview

The project is organized into several key modules, each responsible for different aspects of the game:

- **ai.rs**: Manages the game's narrative flow and state updates, handling player inputs and generating responses.
- **app.rs**: Oversees the main application logic, including game state transitions, input handling, and interaction with the UI.
- **game_state.rs**: Contains the structures and logic for maintaining the current game state, including character sheets and world variables.
- **ui/**: This directory contains the components related to the terminal user interface, including menus, input fields, and the main game screen.
- **character.rs**: Defines the structure and behavior of the characters within the game, including attributes, skills, and inventory management.
- **settings.rs** and **settings_state.rs**: Handle the game's configuration and settings, such as API keys for the game and audio settings.
- **main.rs**: The entry point for the application, initializing the game, and setting up the terminal interface.

The game is built using the [Ratatui](https://github.com/tui-rs-revival/ratatui) library for terminal user interfaces, providing a rich, text-based experience reminiscent of classic text adventures.

## 🚀 Getting Started

1. Clone the repository
2. Install Rust if you haven't already (`https://www.rust-lang.org/tools/install`)
3. Obtain an OpenAI API Key from the [OpenAI website](https://beta.openai.com/signup/)
4. Set your API key using one of these methods:
   - **Option A (Recommended)**: Set environment variable: `export OPENAI_API_KEY=your_api_key_here`
   - **Option B**: Run the game and enter your API key in the settings menu
5. Run `cargo build` to compile the project
6. Start the game with `cargo run`

### Environment Variable Examples

**Unix/Linux/macOS:**

```bash
export OPENAI_API_KEY=sk-openai-key
```

**Windows PowerShell:**

```powershell
$env:OPENAI_API_KEY="sk-openai-key"
```

**Windows Command Prompt:**

```cmd
set OPENAI_API_KEY=sk-openai-key
```

The environment variable takes precedence over the settings file.

## 🤝 How to Contribute

We value every contribution, no matter how small! Here's how you can help:

1. **Fork** the repository
2. Create a new **branch** for your feature
3. Make your changes
4. Submit a **Pull Request** with a clear description of your improvements

Don't worry if you're new to this – I am too and we're learning together! Feel free to ask questions, propose ideas, or even just improve documentation. Every contribution counts!

## 📜 License

Licensed under either of [Apache License, Version 2.0](https://www.youtube.com/watch?v=oHg5SJYRHA0) or [MIT license](https://www.youtube.com/watch?v=oHg5SJYRHA0) at your option.
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## 🙏 Acknowledgements

A huge thank you to everyone who has helped make this project possible, both here on GitHub but also on Reddit and IRL, and to every contributor who joins us on this exciting journey!

Let's create an something cool together! 🎮✨
