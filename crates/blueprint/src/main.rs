//! Blueprint: planning for shows - mic plots, setlists, patch plans and more.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("blueprint {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let open = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(std::path::PathBuf::from);
    blueprint::run(open)
}
