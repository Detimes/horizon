//! Builds a plugin, loads it, rebuilds it with another answer, reloads it: the new code answers,
//! the old state was dropped, and a file that is not a plugin is refused. No server needed.
use horizon_plugin_loader::{Calls, Plugins, Sender};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Default)]
struct Record {
    lines: Vec<String>,
}

impl Calls for Record {
    fn log(&mut self, plugin: &str, line: &str) {
        self.lines.push(format!("log {plugin}: {line}"));
    }
    fn reply(&mut self, _plugin: &str, line: &str) -> bool {
        self.lines.push(format!("reply: {line}"));
        true
    }
    fn tell(&mut self, player: u128, line: &str) -> bool {
        self.lines.push(format!("tell {player}: {line}"));
        true
    }
    fn broadcast(&mut self, line: &str) {
        self.lines.push(format!("broadcast: {line}"));
    }
    fn run_command(&mut self, _line: &str) -> bool {
        false
    }
}

/// A fixture plugin answering `/greet` with `answer`, whose `Drop` writes `dropped` next to
/// the plugins folder.
fn build(work: &Path, answer: &str, version: &str) -> PathBuf {
    build_with(
        work,
        answer,
        version,
        "export_plugin!(Greet, Greet::default());",
    )
}

/// A build for another ABI major: its entry point has another name.
fn build_v2(work: &Path) -> PathBuf {
    build_with(
        work,
        "v2",
        "9.0.0",
        "#[unsafe(no_mangle)] pub extern \"C\" fn horizon_server_plugin_v2() -> *mut PluginV1 { export::<Greet>(Greet::default) }",
    )
}

fn build_with(work: &Path, answer: &str, version: &str, export: &str) -> PathBuf {
    let abi = Path::new(env!("CARGO_MANIFEST_DIR")).join("../abi");
    let src = work.join("fixture");
    std::fs::create_dir_all(src.join("src")).unwrap();
    std::fs::write(
        src.join("Cargo.toml"),
        format!(
            "[package]\nname = \"greet\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[lib]\ncrate-type = [\"cdylib\"]\n\
             [dependencies]\nhorizon-plugin-abi = {{ path = {:?} }}\n[workspace]\n",
            abi.display().to_string()
        ),
    )
    .unwrap();
    let marker = work.join("dropped");
    std::fs::write(
        src.join("src/lib.rs"),
        format!(
            r#"use horizon_plugin_abi::*;
#[derive(Default)]
struct Greet;
impl ServerPlugin for Greet {{
    const NAME: &'static str = "greet";
    const VERSION: &'static str = "{version}";
    fn commands(&self) -> Vec<(&'static str, u8)> {{ vec![("greet", 0), ("secret", 4)] }}
    fn command(&self, cx: &mut Cx<'_>, _s: Option<&PlayerInfo<'_>>, line: &str) -> bool {{
        if line == "boom" {{ panic!("a plugin panic stays in the plugin") }}
        cx.reply("{answer}");
        true
    }}
    fn player_joined(&self, cx: &mut Cx<'_>, p: &PlayerInfo<'_>) {{ cx.tell(p.uuid, &format!("hi {{}}", p.name)); }}
}}
impl Drop for Greet {{ fn drop(&mut self) {{ std::fs::write({marker:?}, "{version}").unwrap(); }} }}
{export}
"#
        ),
    )
    .unwrap();
    let status = Command::new(env!("CARGO"))
        .args(["build", "--quiet", "--offline", "--manifest-path"])
        .arg(src.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", work.join("target"))
        .status()
        .unwrap();
    assert!(status.success());
    work.join("target/debug").join(format!(
        "{}greet{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ))
}

fn install(built: &Path, plugins: &Path, name: &str) {
    std::fs::copy(
        built,
        plugins.join(format!("{name}{}", std::env::consts::DLL_SUFFIX)),
    )
    .unwrap();
}

fn ask(plugins: &Plugins, line: &str, permission: u8) -> Vec<String> {
    let mut record = Record::default();
    let sender = Sender {
        uuid: 7,
        name: "Probe",
        permission,
    };
    plugins.command(&mut record, Some(sender), line);
    record.lines
}

#[test]
fn a_rebuilt_plugin_answers_after_a_reload_without_a_restart() {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("reload");
    let _ = std::fs::remove_dir_all(&work);
    let dir = work.join("plugins");
    std::fs::create_dir_all(&dir).unwrap();

    install(&build(&work, "one", "1.0.0"), &dir, "greet");
    let plugins = Plugins::new(dir.clone());
    assert_eq!(plugins.load_all(), ["loaded greet 1.0.0"]);
    assert_eq!(ask(&plugins, "greet", 0), ["reply: one"]);
    // The level the plugin gave a command is checked before the plugin is asked.
    assert!(ask(&plugins, "secret", 0).is_empty());
    assert_eq!(ask(&plugins, "secret", 4), ["reply: one"]);
    // A panic is caught in the plugin: the command is not taken, the host goes on.
    assert!(ask(&plugins, "boom", 0).is_empty());
    let mut record = Record::default();
    plugins.player_joined(
        &mut record,
        Sender {
            uuid: 7,
            name: "Probe",
            permission: 0,
        },
    );
    assert_eq!(record.lines, ["tell 7: hi Probe"]);

    // Rebuilt in place while loaded, then reloaded: the new code answers, the old state was dropped.
    install(&build(&work, "two", "2.0.0"), &dir, "greet");
    let answer = plugins.admin("plugin reload greet");
    assert!(
        answer.starts_with("Reloaded greet 1.0.0 -> 2.0.0 (load #2)"),
        "{answer}"
    );
    assert_eq!(
        std::fs::read_to_string(work.join("dropped")).unwrap(),
        "1.0.0"
    );
    assert_eq!(ask(&plugins, "greet", 0), ["reply: two"]);
    assert!(
        plugins
            .admin("plugin list")
            .contains("greet 2.0.0 (load #2)")
    );

    // Unloaded: nothing answers; loaded again: the same file answers.
    assert_eq!(plugins.admin("plugin unload greet"), "Unloaded greet");
    assert!(ask(&plugins, "greet", 0).is_empty());
    assert!(
        plugins
            .admin("plugin load greet")
            .starts_with("Loaded greet 2.0.0 (load #3)")
    );
    assert_eq!(ask(&plugins, "greet", 0), ["reply: two"]);

    // A library that is not a plugin, and a plugin under another name, are refused.
    install(&build(&work, "two", "2.0.0"), &dir, "other");
    let answer = plugins.admin("plugin load other");
    assert_eq!(
        answer,
        "other not loaded: the file other holds plugin greet"
    );
    std::fs::write(
        dir.join(format!("junk{}", std::env::consts::DLL_SUFFIX)),
        b"not a library",
    )
    .unwrap();
    assert!(
        plugins
            .admin("plugin load junk")
            .starts_with("junk not loaded: dlopen:")
    );
    plugins.unload_all();
    assert!(ask(&plugins, "greet", 0).is_empty());
}

#[test]
fn a_plugin_of_another_abi_is_refused_and_the_loaded_one_stays() {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("abi");
    let _ = std::fs::remove_dir_all(&work);
    let dir = work.join("plugins");
    std::fs::create_dir_all(&dir).unwrap();
    install(&build(&work, "one", "1.0.0"), &dir, "greet");
    let plugins = Plugins::new(dir.clone());
    plugins.load_all();
    let v2 = build_v2(&work);
    install(&v2, &dir, "greet");
    assert_eq!(
        plugins.admin("plugin reload greet"),
        "greet not reloaded, the loaded one stays: no horizon_server_plugin_v1: not a plugin of ABI 1"
    );
    assert_eq!(ask(&plugins, "greet", 0), ["reply: one"]);
    plugins.unload_all();
}
