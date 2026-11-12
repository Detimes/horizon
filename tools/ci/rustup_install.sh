#!/usr/bin/env bash
# rustup toolchain install with a bounded retry: a reset connection to static.rust-lang.org is transient and hits
# boat and GitHub's runners alike. Four attempts, 10, 20 and 40 s apart; the last failure fails the step.
set -u
install() {
  for delay in 10 20 40 0; do
    rustup "$@" && return 0
    status=$?
    [ "$delay" -eq 0 ] && return "$status"
    echo "rustup $1 failed ($status); retrying in ${delay}s" >&2
    sleep "$delay"
  done
}
install toolchain install "$@" || exit
# The toolchain wasm mods build with (cargo-horizon-mod's pin), which runs the mod API generator, and with it
# rustfmt, inside the workspace's tests. cargo-horizon-mod installs it with the minimal profile when it is missing,
# and a Dyna slot keeps it for later jobs: give it rustfmt and clippy here, a no-op without network when present.
pin=$(sed -n 's/^pub const TOOLCHAIN: &str = "\(.*\)";/\1/p' crates/cargo-horizon-mod/src/wasm.rs 2>/dev/null)
[ -n "$pin" ] || exit 0
if rustup toolchain list | grep -q "^$pin-"; then
  installed=$(rustup component list --installed --toolchain "$pin" 2>/dev/null)
  missing=$(for c in rustfmt clippy; do echo "$installed" | grep -q "^$c-" || echo "$c"; done)
  # shellcheck disable=SC2086 # one word per missing component
  [ -z "$missing" ] || install component add --toolchain "$pin" $missing || exit
elif [ -n "${DYNA_CI_BUILD:-}" ]; then
  # A kept slot: install it once with them, before a test installs it without.
  install toolchain install "$pin" --profile minimal --component rustfmt --component clippy || exit
fi
