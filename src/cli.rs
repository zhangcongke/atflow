use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "at", version, about = "A lightweight @ command palette for Linux terminals")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Menu {
        #[arg(long)]
        shell: bool,
    },
    Recent {
        #[arg(long)]
        shell: bool,
    },
    Flow {
        #[arg(long)]
        shell: bool,
    },
    Search {
        #[arg(long)]
        shell: bool,
        #[arg(value_name = "QUERY")]
        query: Vec<String>,
    },
    Setting,
    Init,
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
