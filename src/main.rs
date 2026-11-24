mod app;
mod editor;
mod system;
mod ui;

use anyhow::Result;
use app::App;
use std::{env, path::PathBuf, process};

const USAGE: &str = "Usage: terracl [FILE|DIR]\n\n  -h, --help       Show this help\n  -V, --version    Show version";

fn main() {
    if let Err(err) = run() {
        eprintln!("terracl: {err:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let arg = env::args_os().nth(1);

    match arg.as_ref().and_then(|value| value.to_str()) {
        Some("--version") | Some("-V") => {
            println!("terracl {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some("--help") | Some("-h") => {
            println!("{USAGE}");
            return Ok(());
        }
        _ => {}
    }

    let mut app = App::new(arg.map(PathBuf::from))?;

    system::terminal::install_panic_hook();
    let mut terminal = system::terminal::setup()?;
    let result = app.run(&mut terminal);
    let restored = system::terminal::restore();

    result?;
    restored
}
