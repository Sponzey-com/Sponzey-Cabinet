use std::path::PathBuf;

use cabinet_platform::release_smoke::{
    CleanInstallSmokeInput, MvpEndToEndSmokeInput, run_clean_install_smoke,
    run_mvp_end_to_end_smoke,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalRunArgs {
    data_dir: PathBuf,
    demo: bool,
}

impl LocalRunArgs {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let _program = args.next();
        let mut data_dir = PathBuf::from(".sponzey-cabinet/local-app");
        let mut demo = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--data-dir" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--data-dir requires a path".to_string())?;
                    data_dir = PathBuf::from(value);
                }
                "--demo" => demo = true,
                "--help" | "-h" => return Err(help_text()),
                unknown => return Err(format!("unknown argument: {unknown}\n\n{}", help_text())),
            }
        }

        Ok(Self { data_dir, demo })
    }
}

fn main() {
    let args = match LocalRunArgs::parse(std::env::args()) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            let code = if message.starts_with("Usage:") { 0 } else { 2 };
            std::process::exit(code);
        }
    };

    if let Err(error) = run_local(args) {
        eprintln!("Sponzey Cabinet local run failed: {error}");
        std::process::exit(1);
    }
}

fn run_local(args: LocalRunArgs) -> Result<(), String> {
    let clean = run_clean_install_smoke(CleanInstallSmokeInput::new(args.data_dir.clone()))
        .map_err(|error| format!("{error:?}"))?;

    println!("Sponzey Cabinet local core ready");
    println!("data_dir={}", args.data_dir.display());
    println!("first_run_completed={}", clean.completed());
    println!("setup_healthy={}", clean.healthy());
    println!("created_directories={}", clean.created_directories());
    println!(
        "already_present_directories={}",
        clean.already_present_directories()
    );

    if args.demo {
        let demo = run_mvp_end_to_end_smoke(MvpEndToEndSmokeInput::new(args.data_dir))
            .map_err(|error| format!("{error:?}"))?;
        println!("mvp_demo=true");
        println!("document_created={}", demo.document_created());
        println!("document_edited={}", demo.document_edited());
        println!("wikilink_parsed={}", demo.wikilink_parsed());
        println!("asset_reference_parsed={}", demo.asset_reference_parsed());
        println!("search_result_found={}", demo.search_result_found());
        println!("backlink_found={}", demo.backlink_found());
        println!("asset_metadata_listed={}", demo.asset_metadata_listed());
        println!("restore_completed={}", demo.restore_completed());
        println!("history_entry_count={}", demo.history_entry_count());
    }

    Ok(())
}

fn help_text() -> String {
    "Usage: cabinet-local [--data-dir PATH] [--demo]\n\nRuns the local Sponzey Cabinet core using an explicit local data directory.".to_string()
}
