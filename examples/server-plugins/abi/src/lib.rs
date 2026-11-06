//! The C ABI between a Horizon dedicated server's plugin host and server plugins built as
//! `cdylib`s (docs/design/server-plugin-reload.md).
//!
//! The host is an ordinary trait mod, built into the server once. A plugin is a shared library
//! in `plugins/` that exports [`ENTRY_SYMBOL`]. Nothing but the `#[repr(C)]` types below crosses
//! the boundary: no Rust type, no allocation, no panic. Strings are borrowed for one call.
//!
//! Versioning: the major version is in the symbol's name, so a plugin of another major is not
//! found at all. Within a major, fields are only ever appended to [`HostV1`] and [`PluginV1`],
//! and each side reads a field only when the other side's `size` covers it.
//!
//! The second half of this file is the plugin side: implement [`ServerPlugin`] and call
//! [`export_plugin!`]; the generated functions catch every panic.
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// The major version this crate speaks.
pub const ABI_MAJOR: u32 = 1;
/// The symbol a plugin exports: `extern "C" fn() -> *mut PluginV1`, null when it failed to start.
pub const ENTRY_SYMBOL: &[u8] = b"horizon_server_plugin_v1\0";

/// A borrowed UTF-8 string, valid for the call it is passed to.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Str {
    pub ptr: *const u8,
    pub len: usize,
}

impl Str {
    pub fn new(s: &str) -> Self {
        Self {
            ptr: s.as_ptr(),
            len: s.len(),
        }
    }

    /// The string, or "" when it is not UTF-8.
    ///
    /// # Safety
    /// `ptr` and `len` describe memory that stays readable for `'a`.
    pub unsafe fn as_str<'a>(self) -> &'a str {
        if self.ptr.is_null() {
            return "";
        }
        let bytes = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
        std::str::from_utf8(bytes).unwrap_or("")
    }
}

/// A player's UUID as two halves (u128 has no stable C layout).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Uuid {
    pub hi: u64,
    pub lo: u64,
}

impl From<u128> for Uuid {
    fn from(v: u128) -> Self {
        Self {
            hi: (v >> 64) as u64,
            lo: v as u64,
        }
    }
}

impl From<Uuid> for u128 {
    fn from(v: Uuid) -> Self {
        ((v.hi as u128) << 64) | v.lo as u128
    }
}

/// A player a hook is about.
#[repr(C)]
pub struct Player {
    pub uuid: Uuid,
    pub name: Str,
    /// Operator level, 0 to 4.
    pub permission: u8,
}

/// One hook call's context inside the host; only passed back to [`HostV1`]'s functions during
/// that call.
#[repr(C)]
pub struct Call {
    _private: [u8; 0],
}

/// What the host offers a plugin, during a hook call.
#[repr(C)]
pub struct HostV1 {
    /// `size_of::<HostV1>()` in the host's build.
    pub size: usize,
    pub major: u32,
    /// A line in the server's log, named after the plugin.
    pub log: extern "C" fn(call: *mut Call, line: Str),
    /// A line of the answer to the command being handled (the player's chat, or the console).
    pub reply: extern "C" fn(call: *mut Call, line: Str) -> bool,
    /// A system chat line for a player; false when they are not on.
    pub tell: extern "C" fn(call: *mut Call, player: Uuid, line: Str) -> bool,
    /// A system chat line for every player.
    pub broadcast: extern "C" fn(call: *mut Call, line: Str),
    /// A command as the server's console; false where the game does not allow it.
    pub run_command: extern "C" fn(call: *mut Call, line: Str) -> bool,
}

/// A loaded plugin, made by its [`ENTRY_SYMBOL`] and owned by the plugin: the host gives it back
/// through `unload` and never frees it itself.
#[repr(C)]
pub struct PluginV1 {
    /// `size_of::<PluginV1>()` in the plugin's build.
    pub size: usize,
    pub major: u32,
    /// Static strings of the plugin.
    pub name: Str,
    pub version: Str,
    pub state: *mut c_void,
    /// Calls `add` once per command usage the plugin answers (`hello [name:word]`), with the
    /// operator level it needs.
    pub commands: extern "C" fn(
        state: *mut c_void,
        sink: *mut c_void,
        add: extern "C" fn(sink: *mut c_void, usage: Str, level: u8),
    ),
    /// A command line without its slash; `sender` is null for the console. True when it was the
    /// plugin's.
    pub command: extern "C" fn(
        state: *mut c_void,
        host: *const HostV1,
        call: *mut Call,
        sender: *const Player,
        line: Str,
    ) -> bool,
    /// A player joined the server.
    pub player_joined: extern "C" fn(
        state: *mut c_void,
        host: *const HostV1,
        call: *mut Call,
        player: *const Player,
    ),
    /// Drops the plugin's state and this struct. No call follows.
    pub unload: extern "C" fn(plugin: *mut PluginV1),
}

/// The entry point's type.
pub type EntryV1 = unsafe extern "C" fn() -> *mut PluginV1;

// ---- Plugin side --------------------------------------------------------------------------

/// A player, as a plugin sees one.
pub struct PlayerInfo<'a> {
    pub uuid: u128,
    pub name: &'a str,
    pub permission: u8,
}

impl PlayerInfo<'_> {
    /// # Safety
    /// `p` comes from the host, for the current call.
    unsafe fn from_raw(p: &Player) -> Self {
        Self {
            uuid: p.uuid.into(),
            name: unsafe { p.name.as_str() },
            permission: p.permission,
        }
    }
}

/// The host, during one hook call.
pub struct Cx<'a> {
    host: &'a HostV1,
    call: *mut Call,
}

impl Cx<'_> {
    pub fn log(&mut self, line: &str) {
        (self.host.log)(self.call, Str::new(line))
    }
    pub fn reply(&mut self, line: &str) -> bool {
        (self.host.reply)(self.call, Str::new(line))
    }
    pub fn tell(&mut self, player: u128, line: &str) -> bool {
        (self.host.tell)(self.call, player.into(), Str::new(line))
    }
    pub fn broadcast(&mut self, line: &str) {
        (self.host.broadcast)(self.call, Str::new(line))
    }
    pub fn run_command(&mut self, line: &str) -> bool {
        (self.host.run_command)(self.call, Str::new(line))
    }
}

/// A server plugin. Its state lives until the host unloads it (`Drop` runs then); state that must
/// survive a reload goes to a file.
pub trait ServerPlugin: Send + Sync + 'static {
    const NAME: &'static str;
    const VERSION: &'static str;
    /// Command usages and the operator level each needs (see `NativeCommand`).
    fn commands(&self) -> Vec<(&'static str, u8)> {
        Vec::new()
    }
    /// True when `line` was this plugin's command.
    fn command(&self, _cx: &mut Cx<'_>, _sender: Option<&PlayerInfo<'_>>, _line: &str) -> bool {
        false
    }
    fn player_joined(&self, _cx: &mut Cx<'_>, _player: &PlayerInfo<'_>) {}
}

/// Builds the [`PluginV1`] for `P`; what [`export_plugin!`] calls.
pub fn export<P: ServerPlugin>(make: impl FnOnce() -> P) -> *mut PluginV1 {
    let Ok(plugin) = catch_unwind(AssertUnwindSafe(make)) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(PluginV1 {
        size: size_of::<PluginV1>(),
        major: ABI_MAJOR,
        name: Str::new(P::NAME),
        version: Str::new(P::VERSION),
        state: Box::into_raw(Box::new(plugin)).cast(),
        commands: commands::<P>,
        command: command::<P>,
        player_joined: player_joined::<P>,
        unload: unload::<P>,
    }))
}

extern "C" fn commands<P: ServerPlugin>(
    state: *mut c_void,
    sink: *mut c_void,
    add: extern "C" fn(*mut c_void, Str, u8),
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let plugin = unsafe { &*(state as *const P) };
        for (usage, level) in plugin.commands() {
            add(sink, Str::new(usage), level);
        }
    }));
}

extern "C" fn command<P: ServerPlugin>(
    state: *mut c_void,
    host: *const HostV1,
    call: *mut Call,
    sender: *const Player,
    line: Str,
) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        let plugin = unsafe { &*(state as *const P) };
        let mut cx = Cx {
            host: unsafe { &*host },
            call,
        };
        let sender = unsafe { sender.as_ref() }.map(|p| unsafe { PlayerInfo::from_raw(p) });
        plugin.command(&mut cx, sender.as_ref(), unsafe { line.as_str() })
    }))
    .unwrap_or(false)
}

extern "C" fn player_joined<P: ServerPlugin>(
    state: *mut c_void,
    host: *const HostV1,
    call: *mut Call,
    player: *const Player,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let plugin = unsafe { &*(state as *const P) };
        let mut cx = Cx {
            host: unsafe { &*host },
            call,
        };
        if let Some(p) = unsafe { player.as_ref() } {
            plugin.player_joined(&mut cx, &unsafe { PlayerInfo::from_raw(p) });
        }
    }));
}

extern "C" fn unload<P: ServerPlugin>(plugin: *mut PluginV1) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let plugin = unsafe { Box::from_raw(plugin) };
        drop(unsafe { Box::from_raw(plugin.state as *mut P) });
    }));
}

/// Exports `$ty`, made by `$make`, as this library's plugin.
#[macro_export]
macro_rules! export_plugin {
    ($ty:ty, $make:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn horizon_server_plugin_v1() -> *mut $crate::PluginV1 {
            $crate::export::<$ty>(|| $make)
        }
    };
}
