#!/usr/bin/env bash
# Install k1fix CLI (Linux / macOS).
# Usage:
#   ./scripts/install.sh
#   curl -fsSL https://raw.githubusercontent.com/<user>/k1fix/master/scripts/install.sh | bash
set -euo pipefail

REPO_URL="${K1FIX_REPO_URL:-https://github.com/k1fix/k1fix.git}"
INSTALL_DIR="${K1FIX_INSTALL_DIR:-${HOME}/.local/bin}"
BRANCH="${K1FIX_BRANCH:-master}"
BIN_NAME="k1fix"

log()  { printf '==> %s\n' "$*"; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

need_cmd() {
  have "$1" || die "falta '$1'. Instalalo y reintentá."
}

find_repo_root() {
  local dir="$1"
  while [[ "$dir" != "/" ]]; do
    if [[ -f "$dir/Cargo.toml" ]] && grep -q 'name = "k1fix"' "$dir/Cargo.toml" 2>/dev/null; then
      printf '%s\n' "$dir"
      return 0
    fi
    dir="$(dirname "$dir")"
  done
  return 1
}

ensure_rust() {
  if have cargo && have rustc; then
    log "Rust OK: $(rustc --version)"
    return 0
  fi
  log "Rust no encontrado. Instalando rustup (default toolchain)…"
  need_cmd curl
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
  have cargo || die "rustup instaló pero cargo no está en PATH. Abrí una shell nueva."
}

resolve_source() {
  local here script_dir
  here="$(pwd)"
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"

  if root="$(find_repo_root "$here")"; then
    printf '%s\n' "$root"
    return 0
  fi
  if root="$(find_repo_root "$script_dir")"; then
    printf '%s\n' "$root"
    return 0
  fi

  need_cmd git
  local cache="${XDG_CACHE_HOME:-$HOME/.cache}/k1fix-src"
  log "Repo local no encontrado. Clonando ${REPO_URL} (${BRANCH}) → ${cache}"
  mkdir -p "$(dirname "$cache")"
  if [[ -d "$cache/.git" ]]; then
    git -C "$cache" fetch --depth 1 origin "$BRANCH"
    git -C "$cache" checkout -q FETCH_HEAD || git -C "$cache" checkout -q "$BRANCH"
    git -C "$cache" pull --ff-only origin "$BRANCH" 2>/dev/null || true
  else
    rm -rf "$cache"
    git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$cache"
  fi
  printf '%s\n' "$cache"
}

main() {
  log "Instalando ${BIN_NAME}…"
  ensure_rust
  need_cmd cargo

  local src
  src="$(resolve_source)"
  log "Fuente: ${src}"

  mkdir -p "$INSTALL_DIR"
  log "Compilando release (puede tardar)…"
  (cd "$src" && cargo build --release --bin "$BIN_NAME")

  local built="${src}/target/release/${BIN_NAME}"
  [[ -x "$built" ]] || die "no se generó ${built}"

  install -m 755 "$built" "${INSTALL_DIR}/${BIN_NAME}"
  log "Binario: ${INSTALL_DIR}/${BIN_NAME}"

  if ! have "$BIN_NAME"; then
    case ":${PATH}:" in
      *":${INSTALL_DIR}:"*) ;;
      *)
        log "Agregá al PATH (zsh/bash):"
        printf '\n  export PATH="%s:$PATH"\n\n' "$INSTALL_DIR"
        log "O añadilo a ~/.bashrc / ~/.zshrc"
        ;;
    esac
  fi

  "${INSTALL_DIR}/${BIN_NAME}" --version || true
  log "Listo. Probá: ${BIN_NAME} profiles list"
}

main "$@"
