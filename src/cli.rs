use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "at",
    version,
    about = "AtFlow, a lightweight @ file flow for Linux terminals"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Flow {
        #[arg(long)]
        shell: bool,
        #[arg(value_name = "QUERY")]
        query: Vec<String>,
    },
    Setting {
        #[arg(long)]
        path: bool,
    },
    Init,
    RecentRecord {
        path: String,
    },
    Shell {
        #[command(subcommand)]
        command: ShellCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ShellCommand {
    Print,
    Hook,
}

impl Command {
    pub fn search_query(query: &[String]) -> Option<String> {
        let joined = query.join(" ");
        let trimmed = joined.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    }
}
