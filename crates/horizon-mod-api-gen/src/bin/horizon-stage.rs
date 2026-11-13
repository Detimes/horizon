//! `horizon-stage ROOT POINTS_JSON INTERNALS_JSON API_VERSION`: expose internals, mark points and give functions their replacement slots in a staged
//! source copy, then write both manifests (#589). Never run on a working tree.
//!
//! `horizon-stage surface ROOT OUT`: write the surface export (`surface.json`) of the source tree
//! at ROOT to OUT, and nothing else. It only reads the tree, so a working tree is fine.
use horizon_mod_api_gen::staging::{self, Mode};
use std::path::PathBuf;

fn surface(root: &std::path::Path, out: &std::path::Path) {
    let started = std::time::Instant::now();
    // The generator writes source paths into `#[path]`s it then reads: they must be absolute.
    let root = match std::fs::canonicalize(root) {
        Ok(root) => root,
        Err(error) => {
            eprintln!("horizon-stage surface: {}: {error}", root.display());
            std::process::exit(1);
        }
    };
    let export = match horizon_mod_api_gen::surface::export(&root) {
        Ok(export) => export,
        Err(error) => {
            eprintln!("horizon-stage surface: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = std::fs::write(out, export.surface.to_json()) {
        eprintln!("horizon-stage surface: {}: {error}", out.display());
        std::process::exit(1);
    }
    for note in &export.notes {
        eprintln!("unclassified: {note}");
    }
    let counts: Vec<String> = export
        .counts()
        .iter()
        .map(|(kind, count)| format!("{kind}={count}"))
        .collect();
    println!(
        "surface {} unclassified={} hash={}",
        counts.join(" "),
        export.notes.len(),
        export.surface.hash
    );
    println!(
        "stage_phase=surface seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
}

fn main() {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if let [command, root, out] = &args[..]
        && command == "surface"
    {
        return surface(root, out);
    }
    let usage = || {
        eprintln!(
            "usage: horizon-stage ROOT POINTS_JSON INTERNALS_JSON API_VERSION [--sync-internals]\n       horizon-stage surface ROOT OUT"
        );
        std::process::exit(2)
    };
    let [root, list, internals, api_version, options @ ..] = &args[..] else {
        usage()
    };
    // Discovery never builds or publishes a game.
    let mode = match options {
        [] => Mode::Build,
        [flag] if flag == "--sync-internals" => Mode::Discovery,
        _ => usage(),
    };
    let Some(api_version) = api_version.to_str() else {
        usage()
    };
    match staging::stage(root, list, internals, mode, api_version) {
        Ok(staging::Staged {
            points: staged,
            replace,
            internals: widened,
        }) => {
            println!(
                "staged {} points ({} with a head only; {} functions have no API record) in {} files",
                staged.points, staged.head_only, staged.refused, staged.files
            );
            println!(
                "gave {} functions a replacement slot in {} files of {} crates",
                replace.functions, replace.files, replace.crates
            );
            let shapes = widened
                .entries
                .iter()
                .filter(|entry| entry.shape_only)
                .count();
            println!(
                "exposed {} internals and listed {} type shapes in {} changed files: {:?}",
                widened.entries.len() - shapes,
                shapes,
                widened.files,
                widened.changed
            );
        }
        Err(error) => {
            eprintln!("horizon-stage: {error}");
            std::process::exit(1);
        }
    }
}
