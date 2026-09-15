use std::env;
use std::error::Error;
use std::process::ExitCode;

use tabsnap_companion::PortableLayout;

fn print_help() {
    println!("TabSnap Companion");
    println!();
    println!("Usage:");
    println!("  tabsnap-companion info   Show portable storage paths");
    println!("  tabsnap-companion init   Create the snapshots directory next to the executable");
}

fn run() -> Result<(), Box<dyn Error>> {
    let command = env::args().nth(1).unwrap_or_else(|| "info".to_owned());
    let layout = PortableLayout::discover()?;

    match command.as_str() {
        "info" => {
            println!("TabSnap Companion");
            println!("mode: portable");
            println!("executable: {}", layout.executable_path().display());
            println!("root: {}", layout.root_dir().display());
            println!("snapshots: {}", layout.snapshots_dir().display());
            println!("initialized: {}", layout.is_initialized());
        }
        "init" => {
            layout.ensure_directories()?;
            println!("Portable storage initialized.");
            println!("snapshots: {}", layout.snapshots_dir().display());
        }
        "help" | "--help" | "-h" => print_help(),
        other => {
            return Err(format!("Unknown command: {other}. Use --help for usage.").into());
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("TabSnap Companion error: {error}");
            ExitCode::FAILURE
        }
    }
}
