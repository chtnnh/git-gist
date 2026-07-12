# git-gist bash helpers
# Usage: source /path/to/shell/gg.bash

# Jump to an alias path from config (requires gg + python3 or gg alias list)
gg-cd() {
  local name="$1"
  if [[ -z "$name" ]]; then
    echo "usage: gg-cd <alias>" >&2
    return 1
  fi
  local path
  path="$(gg alias list 2>/dev/null | awk -v n="$name" -F'\t' '$1==n {print $2; exit}')"
  if [[ -z "$path" ]]; then
    echo "gg-cd: alias not found: $name" >&2
    return 1
  fi
  cd "$path" || return 1
}

# Optional: count dirty child repos for PS1
__gg_dirty_count() {
  gg --only-dirty --color never list 2>/dev/null | wc -l | tr -d ' '
}

gg-prompt() {
  local n
  n="$(__gg_dirty_count)"
  if [[ "$n" != "0" && -n "$n" ]]; then
    echo "[gg:$n dirty]"
  fi
}

# Completions if gg is available
if command -v gg >/dev/null 2>&1; then
  eval "$(gg completions bash 2>/dev/null)" || true
fi
