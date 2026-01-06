use clap::{Parser, Subcommand};
use anyhow::Result;
use flux::commands::{init, status, track, runner, signal, self_update};
use flux::validation::{
    clap_id_validator, clap_name_validator, clap_description_validator, clap_message_validator,
};

#[derive(Parser)]
#[command(name = "flux")]
#[command(about = "Self-propelling agent orchestration CLI", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .work/ directory
    Init,

    /// Show dashboard with context health
    Status,

    /// Check integrity of work directory
    Validate,

    /// Diagnose issues with work directory
    Doctor,

    /// Manage tracks (conversation threads)
    Track {
        #[command(subcommand)]
        command: TrackCommands,
    },

    /// Manage runners (AI agents)
    Runner {
        #[command(subcommand)]
        command: RunnerCommands,
    },

    /// Manage signals (inter-agent communication)
    Signal {
        #[command(subcommand)]
        command: SignalCommands,
    },

    /// Update flux and configuration files
    SelfUpdate,
}

#[derive(Subcommand)]
enum TrackCommands {
    /// Create a new track
    New {
        /// Name of the track (max 64 characters)
        #[arg(value_parser = clap_name_validator)]
        name: String,

        /// Description of the track (max 500 characters)
        #[arg(short, long, value_parser = clap_description_validator)]
        description: Option<String>,
    },

    /// List all tracks
    List {
        /// Show archived tracks
        #[arg(short, long)]
        archived: bool,
    },

    /// Show details of a specific track
    Show {
        /// Track ID or name (max 64 characters)
        #[arg(value_parser = clap_name_validator)]
        id: String,
    },

    /// Close a track
    Close {
        /// Track ID or name (max 64 characters)
        #[arg(value_parser = clap_name_validator)]
        id: String,

        /// Reason for closing (max 500 characters)
        #[arg(short, long, value_parser = clap_description_validator)]
        reason: Option<String>,
    },
}

#[derive(Subcommand)]
enum RunnerCommands {
    /// Create a new runner
    Create {
        /// Runner name (max 64 characters)
        #[arg(value_parser = clap_name_validator)]
        name: String,

        /// Runner type (e.g., sonnet, opus) (max 64 characters)
        #[arg(short, long, value_parser = clap_name_validator)]
        runner_type: String,
    },

    /// List all runners
    List {
        /// Show inactive runners
        #[arg(short, long)]
        inactive: bool,
    },

    /// Assign runner to track
    Assign {
        /// Runner ID (alphanumeric, dash, underscore only; max 128 characters)
        #[arg(value_parser = clap_id_validator)]
        runner: String,

        /// Track ID (alphanumeric, dash, underscore only; max 128 characters)
        #[arg(value_parser = clap_id_validator)]
        track: String,
    },

    /// Release runner from track
    Release {
        /// Runner ID (alphanumeric, dash, underscore only; max 128 characters)
        #[arg(value_parser = clap_id_validator)]
        runner: String,
    },

    /// Archive a runner
    Archive {
        /// Runner ID (alphanumeric, dash, underscore only; max 128 characters)
        #[arg(value_parser = clap_id_validator)]
        runner: String,
    },
}

#[derive(Subcommand)]
enum SignalCommands {
    /// Set a signal for a runner
    Set {
        /// Target runner ID (alphanumeric, dash, underscore only; max 128 characters)
        #[arg(value_parser = clap_id_validator)]
        runner: String,

        /// Signal type (max 64 characters)
        #[arg(value_parser = clap_name_validator)]
        signal_type: String,

        /// Signal message (max 1000 characters)
        #[arg(value_parser = clap_message_validator)]
        message: String,

        /// Priority (1-5)
        #[arg(short, long, default_value = "3")]
        priority: u8,
    },

    /// Show signals for a runner
    Show {
        /// Runner ID (optional, shows all if not specified; max 128 characters)
        #[arg(value_parser = clap_id_validator)]
        runner: Option<String>,
    },

    /// Clear a signal
    Clear {
        /// Signal ID (alphanumeric, dash, underscore only; max 128 characters)
        #[arg(value_parser = clap_id_validator)]
        id: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init::execute(),
        Commands::Status => status::execute(),
        Commands::Validate => status::validate(),
        Commands::Doctor => status::doctor(),
        Commands::Track { command } => match command {
            TrackCommands::New { name, description } => {
                track::create(name, description)
            }
            TrackCommands::List { archived } => {
                track::list(archived)
            }
            TrackCommands::Show { id } => {
                track::show(id)
            }
            TrackCommands::Close { id, reason } => {
                track::close(id, reason)
            }
        },
        Commands::Runner { command } => match command {
            RunnerCommands::Create { name, runner_type } => {
                runner::create(name, runner_type)
            }
            RunnerCommands::List { inactive } => {
                runner::list(inactive)
            }
            RunnerCommands::Assign { runner, track } => {
                runner::assign(runner, track)
            }
            RunnerCommands::Release { runner } => {
                runner::release(runner)
            }
            RunnerCommands::Archive { runner } => {
                runner::archive(runner)
            }
        },
        Commands::Signal { command } => match command {
            SignalCommands::Set { runner, signal_type, message, priority } => {
                signal::set(runner, signal_type, message, priority)
            }
            SignalCommands::Show { runner } => {
                signal::show(runner)
            }
            SignalCommands::Clear { id } => {
                signal::clear(id)
            }
        },
        Commands::SelfUpdate => self_update::execute(),
    }
}
