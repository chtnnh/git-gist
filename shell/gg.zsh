# git-gist zsh helpers
# Usage: source /path/to/shell/gg.zsh

if (( $+aliases[gg] )); then
  print -u2 "git-gist: gg is already an alias; run \"unalias 'gg'\" and remove or rename it in your shell startup file."
  return 1 2>/dev/null || exit 1
fi
if (( $+galiases[gg] )); then
  print -u2 "git-gist: gg is already a global alias; run \"unalias 'gg'\" and remove or rename it in your shell startup file."
  return 1 2>/dev/null || exit 1
fi
if (( $+functions[gg] )); then
  print -u2 "git-gist: gg is already a function; run 'unfunction gg' and remove or rename it in your shell startup file."
  return 1 2>/dev/null || exit 1
fi

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

if (( $+commands[gg] )); then
  eval "$(gg completions zsh 2>/dev/null)" || true
fi
