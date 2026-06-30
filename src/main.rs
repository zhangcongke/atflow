use anyhow::Result;
use at::cli::{Cli, Command, ShellCommand};
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Menu { shell: false }) {
        Command::Menu { shell } => println!("menu shell={shell}"),
        Command::Recent { shell } => println!("recent shell={shell}"),
        Command::Flow { shell } => println!("flow shell={shell}"),
        Command::Search { shell, query } => {
            let query = Command::search_query(&query).unwrap_or_default();
            println!("search shell={shell} query={query}");
        }
        Command::Setting => println!("setting"),
        Command::Init => println!("init"),
        Command::RecentRecord { path } => {
            let db = at::history::HistoryDb::open(&at::history::default_history_path())?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64;
            db.record_path_at(
                std::path::Path::new(&path),
                at::history::PathKind::Dir,
                at::history::HistorySource::ShellCdHook,
                now,
            )?;
        }
        Command::Shell { command } => match command {
            ShellCommand::Print => println!("{}", at::shell::functions_block()),
            ShellCommand::Hook => println!("{}", at::shell::cd_hook_block()),
        },
    }
    Ok(())
}
