//! himd — voice-first companion for Claude Code.
//!
//! This binary is the CLI entrypoint. It parses subcommands and delegates to
//! the MCP server (`himd-mcp`) or diagnostic routines.

mod doctor;

use clap::{Parser, Subcommand};

/// himd: a voice-first companion for Claude Code
#[derive(Parser)]
#[command(name = "himd", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the MCP server on stdio (the primary mode for Claude Code)
    ServeStdio,
    /// Check system dependencies and configuration
    Doctor {
        /// Output diagnostics as JSON for machine consumption
        #[arg(long)]
        json: bool,
    },
    /// Capture audio from the default microphone
    Capture {
        /// Maximum recording duration in seconds (default 30, max 60)
        #[arg(long)]
        max_duration_secs: Option<f64>,
    },
    /// Analyze a local audio file
    Analyze {
        /// Path to the audio file to analyze
        file: String,
    },
    /// Synthesize speech and play it aloud
    Say {
        /// Text to speak (max 600 chars)
        text: String,
        /// Voice name to use
        #[arg(long)]
        voice: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::ServeStdio => {
            himd_mcp::serve_stdio().await?;
        }
        Command::Doctor { json } => {
            let report = doctor::run_diagnostics();
            if json {
                doctor::print_json(&report);
            } else {
                doctor::print_human(&report);
            }
            if !report.readiness.pass {
                std::process::exit(1);
            }
        }
        Command::Capture { max_duration_secs } => {
            let result = himd_audio::capture::capture_once_blocking(max_duration_secs)?;
            println!("{}", serde_json::to_string(&result).unwrap());
        }
        Command::Analyze { file } => {
            let provider_result = himd_core::provider::understand(&file).await?;
            let turn = himd_core::acoustic::build_audio_turn(&provider_result, &file);
            println!("{}", serde_json::to_string(&turn).unwrap());
        }
        Command::Say { text, voice } => {
            let result = himd_core::tts::synthesize(&text, voice, None, None).await?;
            himd_audio::playback::play_file(std::path::Path::new(&result.audio_file))?;
            println!("{}", result.audio_file);
        }
    }

    Ok(())
}
