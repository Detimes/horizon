# Server plugins changed without a restart

A dedicated server's admin wants to change a plugin, rebuild it, and see the new code run while players stay
on. Today a native mod's code is in the server's binary: a change is a game rebuild (minutes, see
[mod-hotload.md](mod-hotload.md#measured)) and a restart that disconnects everyone. This note describes a
plugin host that loads server plugins as shared libraries from `plugins/` and reloads one with
`/plugin reload <name>`, and the prototype in `examples/server-plugins/`.

## Where it sits next to the plan

- [#7386](https://github.com/horizon-rust/horizon/issues/7386) settled native mods on **Rust `dylib`s**
  built by the launcher against a `dylib` build of the game, loaded at start and **never unloaded**, with
  "rebuild the mod, restart the game" as the development loop. Its stages 3 (spike) and 4 (launcher and
  loader) are arismoko's; nothing here touches them.
- [mod-hotload.md](mod-hotload.md) weighs option D, "a C ABI or `abi_stable` plugin layer over
  `native-hooks.json`", and keeps it out of Horizon: a second API beside `mod_api`, still not sandboxed.
  Option A, a trait mod switched off and on at run time, is in (#10218).
- #7386 also lists "scripting runtimes (Lua, JS, WASM) shipped as ordinary mods" as something its design
  enables. The plugin host is that kind of thing: **an ordinary trait mod**, built into the server once,
  whose own business is loading code. Horizon gets no second API and no change; the ABI belongs to the
  host and its plugins.

## Pieces

| Piece | What | Built |
|---|---|---|
| `abi/` (`horizon-plugin-abi`) | the `#[repr(C)]` tables both sides share, and the plugin-side SDK (`ServerPlugin`, `export_plugin!`) | with each side; no dependencies |
| `loader/` (`horizon-plugin-loader`) | finds, maps, starts, reloads and unloads plugins; dispatches commands and joins; engine-free, tested without a server | into the host |
| `host/` (`pluginhost`) | a trait mod (`[api]` package): `Hooks` to loader calls, loader calls to `Host` | into the server, once |
| a plugin (`hello/`) | a `cdylib` | alone, in about a second |

## The ABI

A plugin exports one symbol, `horizon_server_plugin_v1`, of type `extern "C" fn() -> *mut PluginV1`:

- `PluginV1`: `size`, `major`, the plugin's `name` and `version`, an opaque `state`, and four functions:
  `commands` (its usages and operator levels, through a callback), `command` (a line without its slash and
  its sender, null for the console; true when taken), `player_joined`, and `unload` (drops the state and the
  table; nothing follows).
- `HostV1`, passed to each call with an opaque `Call` for that call only: `log`, `reply` (the answer to the
  command being handled), `tell`, `broadcast`, `run_command`.
- Only `#[repr(C)]` values cross: borrowed UTF-8 strings (`Str`, valid for the call), a UUID as two `u64`s,
  plain integers. No `String`, `Box`, `Vec` or Rust trait object crosses, and neither side frees the other's
  memory, so the plugin's own `std` and allocator (a `cdylib` links its own) never meet the game's.
- Panics stop at the boundary: the SDK's generated functions wrap each call in `catch_unwind`, and a panic
  becomes "not handled".

**Versions.** The major is in the symbol's name, so a plugin of another major is not found: the host says
"not a plugin of ABI 1" and calls nothing of it. Within a major, fields are only appended to either table;
each side reads a field only when the other side's `size` covers it, so a plugin built against an older
minor runs on a newer host and the other way round (a plugin checks `HostV1::size` before using a newer
host function). The plugin's `major` and `size` are checked again after its entry point runs.

**Crates.** `libloading` 0.8.9 is in Horizon's `Cargo.lock` already (`horizon-narrator`, `wgpu-hal`), and so
is `log`; the ABI crate uses only `std`. So the host builds under today's rule that a mod uses only locked
crates, and **nothing needs adding to Horizon**. A plugin is its own Cargo build with its own lock: it may
use crates the game doesn't ship. (Packaging: the launcher copies only a bundle's `[api]` folder, so a
released host carries the loader and the ABI inside that folder; `stage.py` stages them as workspace
members instead.)

## Load, reload, unload

- **Start.** The first hook that needs them (`register_commands`, a join, a command) loads every
  `plugins/<name>.so` beside `server.properties`.
- **Load.** The file is copied to `plugins/.loaded/<name>-<n>.so`, mapped (`dlopen`), and the copy's name is
  removed at once (the mapping keeps it). Two reasons: the dynamic loader hands out the library already
  mapped from a path, so loading the same path again would give the old code; and the admin may overwrite
  `plugins/<name>.so` with a new build while the old one runs. Then the entry symbol, the entry point, the
  `major` and `size`, and the plugin's name against the file's.
- **Reload.** The new file is mapped first: one that is not a plugin of this ABI leaves the running one in
  place. Then the old plugin's `unload` (its `Drop` runs), then the new entry point. A plugin never runs twice
  at once. Measured below: a few milliseconds.
- **Unload.** `unload`, then the host stops calling it. **Its library stays mapped until the process
  exits.** Unmapping Rust code is not sound: a thread it started, a thread-local's destructor or a `'static`
  it handed out would then point into freed memory. This is the same conclusion as #7386 and mod-hotload.md;
  the difference is only that a fresh copy can be loaded beside the old one. Each reload costs one mapping
  (the example: 426 KB).
- **`/plugin list | load | unload | reload <name>`**: operator level 4 and the console. A plugin that runs
  `/plugin` from its own command gets "busy" instead of a deadlock.

## What a plugin has to respect

- Calls come on the server thread, during a hook; `Cx` (the host for that call) can't be kept or sent.
- State is dropped on reload. What must survive goes to a file (the config folder, the world's folder).
- Threads and sockets a plugin opens are its own: it must stop and join them in `Drop`, or they keep running
  the old copy (safe, since it stays mapped, but leaking).
- Native plugins are full-trust code with the server's rights, as native mods are. Nothing is sandboxed.

## What can be swapped live and what can't

| Change | Live? |
|---|---|
| A plugin's code, its answers, its reaction to the hooks the host forwards | **yes**, `/plugin reload` |
| A new plugin, or one removed | **yes**, `/plugin load` / `unload` |
| A new command | runs at once for level-0 commands (through `player_command`), but players' clients complete it only after a restart, and an operator-only command waits for the restart: the game builds each world's command tree once (`register_native_commands`) |
| A hook the host doesn't forward yet | host rebuild (engine) **once per hook kind**, then every plugin uses it live |
| An injection into engine internals (a mixin `HEAD` into `lan`, `hub`, `conn`...) | **no**: stays a compiled bundle |
| Registries (blocks, items, entity types, effects) | no, as for every mod (mod-hotload.md, "World reload") |
| The ABI or the host itself | engine rebuild and restart |

## How a port moves onto it

The plugin ports written so far are horizon-mixins bundles: an engine-free `core` crate (model, storage,
commands) plus glue helpers injected at `HEAD` of catalog hooks. Across ten bundles that is 69 injections
into catalog hooks and 13 into engine internals. A port moves in three steps:

1. The `core` crate stays as it is and becomes a dependency of a `cdylib` plugin crate.
2. The glue (one helper per hook) becomes one `ServerPlugin` method per hook. Helpers that reach into `Game`
   (`native_server_players`, `native_run_command`, `native_tell`...) become `HostV1` functions; each one the
   host gains is one appended field and one line of glue in the host.
3. Injections into engine internals stay in a small compiled bundle beside the plugin, or wait for a hook.

The first candidates are the bundles with no internal injections at all (permissions: 12 catalog hooks;
chunk pre-generation: 9; the chat bridge: 10); a few with one internal injection follow with a shim.

## If #7386's loader learns to reload

The same pattern works for its Rust `dylib`s on a dedicated server: map a fresh copy of the mod's `dylib`
under a new name, replace its `Box<dyn Hooks>` in dispatch, keep the old copy mapped. With a shared `std`
and allocator it needs no C ABI at all, and trait mods would get `/mod reload` with their `Hooks` as they
are. What that needs in Horizon: the process-wide mod list (`mod_api::MODS`, a `OnceLock<Arc<[...]>>` read
on every dispatch) replaceable between ticks, and a way to rebuild the command trees. The second is useful
today on its own (the "new command" row above) and is the one small request this note makes.

## Measured

On the shared build machine (6 cores, other builds running), 2026-11-06:

| Step | Time |
|---|---|
| Rebuild the example plugin after a one-line change (`cargo build --release -p hello`) | see the proof |
| `/plugin reload hello` (map, unload, start) | see the proof |
| The same change as a compiled native mod: engine rebuild, then restart | 3 to 10 min (mod-hotload.md) plus the restart |

## Prototype

`examples/server-plugins/`: `abi/`, `loader/` (with `tests/reload.rs`: builds a fixture plugin, loads it,
rebuilds it with another answer while loaded, reloads it, checks levels, a panic, unload and load, a library
under the wrong name, a file that is not a library and a plugin of another ABI), `host/` and `hello/`.
`stage.py` stages a dedicated server with the host linked in, as the launcher's generated executable does for
a trait mod.

Not done: Windows (`DLL_SUFFIX` is used, but copies can't be unlinked while mapped and the copy is not cleaned
up), forwarding hooks beyond commands and joins, `tab_complete` for plugin commands, a file watcher
(reload when `plugins/<name>.so` changes), and the command-tree refresh.
