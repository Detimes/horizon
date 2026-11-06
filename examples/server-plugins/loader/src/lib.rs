//! Loads server plugins (`cdylib`s speaking `horizon-plugin-abi`) from a folder and reloads them
//! while the server runs. Engine-free: the plugin host (a trait mod) hands it a [`Calls`] for each
//! hook call, so this crate is tested without a server (tests/reload.rs).
//!
//! A reload drops the old plugin's state, stops calling it and loads the new file. The old library
//! stays mapped until the process exits: unmapping Rust code is not sound (a thread, a
//! thread-local's destructor or a `'static` it handed out would point into freed memory), so each
//! reload costs the size of one copy of the library.
use horizon_plugin_abi as abi;
use std::collections::BTreeSet;
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::Instant;

/// What the game does for a plugin during one hook call.
pub trait Calls {
    fn log(&mut self, plugin: &str, line: &str);
    /// A line of the answer to the command being handled.
    fn reply(&mut self, plugin: &str, line: &str) -> bool;
    fn tell(&mut self, player: u128, line: &str) -> bool;
    fn broadcast(&mut self, line: &str);
    fn run_command(&mut self, line: &str) -> bool;
}

/// Who sent a command; `None` is the console.
#[derive(Clone, Copy)]
pub struct Sender<'a> {
    pub uuid: u128,
    pub name: &'a str,
    pub permission: u8,
}

pub struct Plugins {
    dir: PathBuf,
    list: RwLock<Vec<Loaded>>,
    /// Command roots in the players' command tree, which the game builds once.
    registered: Mutex<BTreeSet<String>>,
    loads: AtomicU32,
}

struct Loaded {
    name: String,
    version: String,
    load: u32,
    commands: Vec<(String, u8)>,
    plugin: *mut abi::PluginV1,
    /// Never dropped: see the crate comment.
    _library: ManuallyDrop<libloading::Library>,
}

// The plugin's state is `Send + Sync` by the SDK's `ServerPlugin` bound; the pointer is only
// passed back to its own functions.
unsafe impl Send for Loaded {}
unsafe impl Sync for Loaded {}

struct CallCx<'a> {
    calls: &'a mut dyn Calls,
    plugin: &'a str,
}

static HOST: abi::HostV1 = abi::HostV1 {
    size: size_of::<abi::HostV1>(),
    major: abi::ABI_MAJOR,
    log: host_log,
    reply: host_reply,
    tell: host_tell,
    broadcast: host_broadcast,
    run_command: host_run_command,
};

/// # Safety
/// `call` is the `CallCx` of the hook call in progress.
unsafe fn cx<'x>(call: *mut abi::Call) -> &'x mut CallCx<'x> {
    unsafe { &mut *(call as *mut CallCx<'x>) }
}

extern "C" fn host_log(call: *mut abi::Call, line: abi::Str) {
    let cx = unsafe { cx(call) };
    cx.calls.log(cx.plugin, unsafe { line.as_str() });
}

extern "C" fn host_reply(call: *mut abi::Call, line: abi::Str) -> bool {
    let cx = unsafe { cx(call) };
    cx.calls.reply(cx.plugin, unsafe { line.as_str() })
}

extern "C" fn host_tell(call: *mut abi::Call, player: abi::Uuid, line: abi::Str) -> bool {
    unsafe { cx(call) }
        .calls
        .tell(player.into(), unsafe { line.as_str() })
}

extern "C" fn host_broadcast(call: *mut abi::Call, line: abi::Str) {
    unsafe { cx(call) }
        .calls
        .broadcast(unsafe { line.as_str() })
}

extern "C" fn host_run_command(call: *mut abi::Call, line: abi::Str) -> bool {
    unsafe { cx(call) }
        .calls
        .run_command(unsafe { line.as_str() })
}

extern "C" fn add_command(sink: *mut std::ffi::c_void, usage: abi::Str, level: u8) {
    let list = unsafe { &mut *(sink as *mut Vec<(String, u8)>) };
    list.push((unsafe { usage.as_str() }.to_owned(), level.min(4)));
}

/// A command line's first word.
pub fn root(line: &str) -> &str {
    line.split(' ').next().unwrap_or_default()
}

fn raw(sender: Sender<'_>) -> abi::Player {
    abi::Player {
        uuid: sender.uuid.into(),
        name: abi::Str::new(sender.name),
        permission: sender.permission,
    }
}

impl Loaded {
    fn plugin(&self) -> &abi::PluginV1 {
        unsafe { &*self.plugin }
    }

    fn unload(self) {
        (self.plugin().unload)(self.plugin);
        // `_library` is ManuallyDrop: the code stays mapped.
    }
}

impl Plugins {
    /// Plugins in `dir` (`<name>.so`), none loaded yet.
    pub fn new(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(dir.join(".loaded"));
        Self {
            dir,
            list: RwLock::default(),
            registered: Mutex::default(),
            loads: AtomicU32::new(0),
        }
    }

    /// Loads every plugin file in the folder; one line per file.
    pub fn load_all(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                name.strip_suffix(std::env::consts::DLL_SUFFIX)
                    .map(str::to_owned)
            })
            .collect();
        names.sort();
        let mut list = self.list.write().unwrap();
        names
            .into_iter()
            .map(|name| match self.open(&name) {
                Ok(p) => {
                    let line = format!("loaded {} {}", p.name, p.version);
                    list.push(p);
                    line
                }
                Err(e) => format!("{name} not loaded: {e}"),
            })
            .collect()
    }

    /// Every loaded plugin's command usages and levels.
    pub fn commands(&self) -> Vec<(String, u8)> {
        let list = self.list.read().unwrap();
        list.iter()
            .flat_map(|p| p.commands.iter().cloned())
            .collect()
    }

    /// Notes that the game's command tree has these roots.
    pub fn mark_registered(&self, roots: impl IntoIterator<Item = String>) {
        self.registered.lock().unwrap().extend(roots);
    }

    pub fn is_registered(&self, root: &str) -> bool {
        self.registered.lock().unwrap().contains(root)
    }

    /// Maps `<dir>/<name>.so` through a copy of its own, so the file can be rebuilt in place
    /// while the copy is mapped, and a second load is a new library, not the first one again
    /// (the dynamic loader hands out the library already loaded from a path). Runs none of the
    /// plugin's code but its library's initializers; a plugin of another ABI fails here.
    fn map(&self, name: &str) -> Result<(libloading::Library, abi::EntryV1, u32), String> {
        let file = self
            .dir
            .join(format!("{name}{}", std::env::consts::DLL_SUFFIX));
        if !file.is_file() {
            return Err(format!("no file {}", file.display()));
        }
        let load = self.loads.fetch_add(1, Ordering::Relaxed) + 1;
        let copy = self
            .dir
            .join(".loaded")
            .join(format!("{name}-{load}{}", std::env::consts::DLL_SUFFIX));
        std::fs::copy(&file, &copy).map_err(|e| format!("copy: {e}"))?;
        let library = unsafe { libloading::Library::new(&copy) };
        // The mapping keeps the copy's contents; the name is not needed any more.
        #[cfg(unix)]
        let _ = std::fs::remove_file(&copy);
        let library = library.map_err(|e| format!("dlopen: {e}"))?;
        let entry = unsafe { library.get::<abi::EntryV1>(abi::ENTRY_SYMBOL).map(|s| *s) };
        match entry {
            Ok(entry) => Ok((library, entry, load)),
            Err(_) => {
                // Never unmapped either: its initializers ran.
                std::mem::forget(library);
                Err("no horizon_server_plugin_v1: not a plugin of ABI 1".into())
            }
        }
    }

    fn open(&self, name: &str) -> Result<Loaded, String> {
        let mapped = self.map(name)?;
        self.start(name, mapped)
    }

    /// Runs the plugin's entry point.
    fn start(
        &self,
        name: &str,
        (library, entry, load): (libloading::Library, abi::EntryV1, u32),
    ) -> Result<Loaded, String> {
        let library = ManuallyDrop::new(library);
        let plugin = unsafe { entry() };
        if plugin.is_null() {
            return Err("its entry point failed".into());
        }
        let p = unsafe { &*plugin };
        if p.major != abi::ABI_MAJOR || p.size < size_of::<abi::PluginV1>() {
            // Another ABI's struct: nothing else of it is called.
            return Err(format!(
                "built for plugin ABI {} (size {}), this host speaks {} (size {})",
                p.major,
                p.size,
                abi::ABI_MAJOR,
                size_of::<abi::PluginV1>()
            ));
        }
        let mut commands = Vec::new();
        (p.commands)(
            p.state,
            (&mut commands as *mut Vec<(String, u8)>).cast(),
            add_command,
        );
        let loaded = Loaded {
            name: unsafe { p.name.as_str() }.to_owned(),
            version: unsafe { p.version.as_str() }.to_owned(),
            load,
            commands,
            plugin,
            _library: library,
        };
        if loaded.name != name {
            let declared = loaded.name.clone();
            loaded.unload();
            return Err(format!("the file {name} holds plugin {declared}"));
        }
        Ok(loaded)
    }

    /// `plugin <list|load|unload|reload> [name]`, the answer as one line.
    pub fn admin(&self, line: &str) -> String {
        let mut words = line.split_whitespace().skip(1);
        let (action, name) = (words.next().unwrap_or("list"), words.next());
        // A plugin's own command may run `/plugin`: the list is borrowed then.
        let Ok(mut list) = self.list.try_write() else {
            return "Plugins are busy; try again".into();
        };
        let at = name.and_then(|n| list.iter().position(|p| p.name == n));
        match (action, name) {
            ("list", _) => {
                let names: Vec<_> = list
                    .iter()
                    .map(|p| format!("{} {} (load #{})", p.name, p.version, p.load))
                    .collect();
                format!("Plugins ({}): {}", names.len(), names.join(", "))
            }
            ("unload", Some(n)) => match at {
                Some(i) => {
                    list.remove(i).unload();
                    format!("Unloaded {n}")
                }
                None => format!("{n} is not loaded"),
            },
            ("load" | "reload", Some(n)) => {
                if action == "load" && at.is_some() {
                    return format!("{n} is loaded already; use /plugin reload {n}");
                }
                let started = Instant::now();
                // The new file is mapped first: one that is not a plugin of this ABI leaves the old
                // one running.
                let mapped = match self.map(n) {
                    Ok(m) => m,
                    Err(e) if at.is_some() => {
                        return format!("{n} not reloaded, the loaded one stays: {e}");
                    }
                    Err(e) => return format!("{n} not loaded: {e}"),
                };
                // Then the old one goes: a plugin never runs twice at once.
                let old = at.map(|i| list.remove(i));
                let from = old
                    .as_ref()
                    .map(|p| format!("{} -> ", p.version))
                    .unwrap_or_default();
                if let Some(old) = old {
                    old.unload();
                }
                match self.start(n, mapped) {
                    Ok(p) => {
                        let new: Vec<_> = p
                            .commands
                            .iter()
                            .map(|(u, _)| root(u).to_owned())
                            .filter(|r| !self.is_registered(r))
                            .collect();
                        let mut answer = format!(
                            "{} {n} {from}{} (load #{}) in {} ms",
                            if from.is_empty() {
                                "Loaded"
                            } else {
                                "Reloaded"
                            },
                            p.version,
                            p.load,
                            started.elapsed().as_millis()
                        );
                        if !new.is_empty() {
                            answer += &format!(
                                "; new commands {} work for everyone but complete after a restart",
                                new.join(", ")
                            );
                        }
                        list.push(p);
                        answer
                    }
                    Err(e) => format!("{n} not loaded: {e}"),
                }
            }
            _ => "Usage: /plugin list | load <name> | unload <name> | reload <name>".into(),
        }
    }

    /// A command for the plugins: true when one took it. `sender`'s level is checked against the
    /// level the plugin gave the command.
    pub fn command(&self, calls: &mut dyn Calls, sender: Option<Sender<'_>>, line: &str) -> bool {
        let Ok(list) = self.list.read() else {
            return false;
        };
        let player = sender.map(raw);
        for p in list.iter() {
            let allowed = p.commands.iter().any(|(usage, level)| {
                root(usage) == root(line) && sender.is_none_or(|s| s.permission >= *level)
            });
            if !allowed {
                continue;
            }
            let mut cx = CallCx {
                calls: &mut *calls,
                plugin: &p.name,
            };
            let raw = p.plugin();
            let took = (raw.command)(
                raw.state,
                &HOST,
                (&mut cx as *mut CallCx<'_>).cast(),
                player
                    .as_ref()
                    .map_or(std::ptr::null(), |s| s as *const abi::Player),
                abi::Str::new(line),
            );
            if took {
                return true;
            }
        }
        false
    }

    /// A player joined: every plugin hears it.
    pub fn player_joined(&self, calls: &mut dyn Calls, player: Sender<'_>) {
        let Ok(list) = self.list.read() else {
            return;
        };
        let player = raw(player);
        for p in list.iter() {
            let mut cx = CallCx {
                calls: &mut *calls,
                plugin: &p.name,
            };
            let raw = p.plugin();
            (raw.player_joined)(
                raw.state,
                &HOST,
                (&mut cx as *mut CallCx<'_>).cast(),
                &player,
            );
        }
    }

    /// Unloads every plugin (the server stops).
    pub fn unload_all(&self) {
        if let Ok(mut list) = self.list.write() {
            for p in list.drain(..) {
                p.unload();
            }
        }
    }
}
