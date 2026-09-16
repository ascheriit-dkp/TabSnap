pub mod capture;
pub mod coordination;
pub mod library;
pub mod protocol;
pub mod ui;

use std::env;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use capture::{CaptureJobStatus, CaptureTargetStateView};
use library::SnapshotLibrary;
use protocol::ProtocolServer;
use tabsnap_companion::{
    PortableLayout, ResolvedStorage, StorageMode, activate_storage, load_storage_mode,
    resolve_storage, validate_storage_dir,
};

fn print_help() {
    println!("TabSnap Companion");
    println!();
    println!("Usage:");
    println!("  tabsnap-companion info");
    println!("  tabsnap-companion init");
    println!("  tabsnap-companion ui");
    println!("  tabsnap-companion serve");
    println!("  tabsnap-companion capture");
    println!("  tabsnap-companion storage show");
    println!("  tabsnap-companion storage set portable");
    println!("  tabsnap-companion storage set local");
    println!("  tabsnap-companion storage set custom <absolute-path>");
    println!("  tabsnap-companion library list");
    println!("  tabsnap-companion library import <snapshot.tabsnap>");
    println!("  tabsnap-companion library export <file-name.tabsnap> <destination-directory>");
}

fn print_storage(layout: &PortableLayout, storage: &ResolvedStorage) {
    println!("mode: {}", storage.mode.name());
    println!("config: {}", layout.config_path().display());
    println!("snapshots: {}", storage.snapshots_dir.display());
    println!("drive-hint: {}", storage.drive_hint.name());
    println!("initialized: {}", storage.snapshots_dir.is_dir());
}

fn show_info(layout: &PortableLayout) -> Result<(), Box<dyn Error>> {
    let mode = load_storage_mode(layout)?;
    let storage = resolve_storage(layout, &mode)?;

    println!("TabSnap Companion");
    println!("executable: {}", layout.executable_path().display());
    println!("portable-root: {}", layout.root_dir().display());
    print_storage(layout, &storage);
    Ok(())
}

fn parse_storage_mode(args: &[String]) -> Result<StorageMode, Box<dyn Error>> {
    match args.get(3).map(String::as_str) {
        Some("portable") => Ok(StorageMode::Portable),
        Some("local") => Ok(StorageMode::Local),
        Some("custom") => {
            let path = args
                .get(4)
                .ok_or("Custom storage requires an absolute path.")?;
            Ok(StorageMode::Custom(PathBuf::from(path)))
        }
        Some(other) => Err(format!("Unknown storage mode: {other}.").into()),
        None => Err("Missing storage mode. Use portable, local or custom.".into()),
    }
}

fn snapshot_library(layout: &PortableLayout) -> Result<SnapshotLibrary, Box<dyn Error>> {
    let mode = load_storage_mode(layout)?;
    let storage = resolve_storage(layout, &mode)?;
    Ok(SnapshotLibrary::new(storage.snapshots_dir))
}

fn run_library_command(layout: &PortableLayout, args: &[String]) -> Result<(), Box<dyn Error>> {
    let library = snapshot_library(layout)?;

    match args.get(2).map(String::as_str) {
        Some("list") => {
            let entries = library.list()?;
            println!("library: {}", library.root().display());
            println!("snapshots: {}", entries.len());
            for entry in entries {
                println!("{}\t{}", entry.size, entry.file_name);
            }
        }
        Some("import") => {
            let source = args.get(3).ok_or("Import requires a .tabsnap file path.")?;
            let entry = library.import_file(Path::new(source))?;
            println!("Imported opaque encrypted snapshot.");
            println!("file: {}", entry.file_name);
            println!("bytes: {}", entry.size);
            println!("path: {}", entry.path.display());
        }
        Some("export") => {
            let file_name = args
                .get(3)
                .ok_or("Export requires a library .tabsnap file name.")?;
            let destination = args
                .get(4)
                .ok_or("Export requires a destination directory.")?;
            let exported = library.export_file(file_name, Path::new(destination))?;
            println!("Exported opaque encrypted snapshot.");
            println!("path: {}", exported.display());
        }
        Some(other) => return Err(format!("Unknown library command: {other}.").into()),
        None => return Err("Missing library command. Use list, import or export.".into()),
    }

    Ok(())
}

fn serve(layout: &PortableLayout) -> Result<(), Box<dyn Error>> {
    let library = snapshot_library(layout)?;
    validate_storage_dir(library.root())?;
    let server = ProtocolServer::bind(library)?;

    println!("TabSnap Companion protocol v1");
    println!("endpoint: {}", server.endpoint()?);
    println!("pairing-code: {}", server.pairing_code()?);
    println!("binding: IPv4 loopback only");
    println!("session: ephemeral; pairing token is not written to disk");
    println!("Press Ctrl+C or close this window to stop the companion protocol.");

    server.serve_forever()?;
    Ok(())
}

fn print_capture_status(status: &CaptureJobStatus) {
    for target in &status.targets {
        let state = match target.state {
            CaptureTargetStateView::Pending => "pending".to_owned(),
            CaptureTargetStateView::Claimed => "capturing".to_owned(),
            CaptureTargetStateView::Complete { bytes } => format!("complete ({bytes} bytes)"),
            CaptureTargetStateView::Failed { reason } => {
                format!("failed ({})", reason.as_str())
            }
        };
        println!(
            "{}\t{}\t{}",
            target.instance.browser.as_str(),
            target.instance.instance_id,
            state
        );
    }
}

fn run_capture_command(layout: &PortableLayout) -> Result<(), Box<dyn Error>> {
    let library = snapshot_library(layout)?;
    validate_storage_dir(library.root())?;
    let server = ProtocolServer::bind(library)?;
    let control = server.capture_control();

    println!("TabSnap coordinated capture v1");
    println!("endpoint: {}", server.endpoint()?);
    println!("pairing-code: {}", server.pairing_code()?);
    println!("Connect the browser pages you want to capture, then press Enter.");

    thread::spawn(move || {
        if let Err(error) = server.serve_forever() {
            eprintln!("TabSnap Companion protocol error: {error}");
        }
    });

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let created = control.create_capture_job()?;
    println!("capture-job: {}", created.job_id);
    print_capture_status(&created);

    let mut previous = created;
    loop {
        thread::sleep(Duration::from_millis(250));
        let Some(current) = control.capture_job_status(&previous.job_id)? else {
            return Err("Capture job expired before completion.".into());
        };
        if current != previous {
            println!();
            print_capture_status(&current);
        }
        if current.is_terminal() {
            println!();
            println!(
                "capture complete: {} succeeded, {} failed",
                current.completed_count(),
                current.failed_count()
            );
            println!(
                "M30 results are held in memory only; M31 adds the machine snapshot container."
            );
            return Ok(());
        }
        previous = current;
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("info");
    let layout = PortableLayout::discover()?;

    match command {
        "info" => show_info(&layout)?,
        "init" => {
            let mode = load_storage_mode(&layout)?;
            let storage = resolve_storage(&layout, &mode)?;
            validate_storage_dir(&storage.snapshots_dir)?;
            println!("Storage initialized and writable.");
            print_storage(&layout, &storage);
        }
        "ui" => ui::run()?,
        "serve" => serve(&layout)?,
        "capture" => run_capture_command(&layout)?,
        "storage" => match args.get(2).map(String::as_str) {
            Some("show") => {
                let mode = load_storage_mode(&layout)?;
                let storage = resolve_storage(&layout, &mode)?;
                print_storage(&layout, &storage);
            }
            Some("set") => {
                let mode = parse_storage_mode(&args)?;
                let storage = activate_storage(&layout, mode)?;
                println!("Storage mode updated and write-tested.");
                print_storage(&layout, &storage);
            }
            Some(other) => {
                return Err(format!("Unknown storage command: {other}.").into());
            }
            None => return Err("Missing storage command. Use show or set.".into()),
        },
        "library" => run_library_command(&layout, &args)?,
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
