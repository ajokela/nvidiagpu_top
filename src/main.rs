mod app;
mod data;
mod parser;
mod process;
mod ui;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "nvidiagpu_top")]
#[command(about = "A TUI for monitoring NVIDIA GPU metrics", long_about = None)]
struct Args {
    /// History retention in seconds
    #[arg(long, default_value = "300")]
    history: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize terminal
    let terminal = ratatui::init();

    // Run app
    let app = app::App::new(args.history);
    let result = app.run(terminal).await;

    // Restore terminal
    ratatui::restore();

    result
}
