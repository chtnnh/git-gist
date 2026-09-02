# git-gist POSIX sh helpers (minimal)
# Usage: . /path/to/shell/gg.sh

if alias gg >/dev/null 2>&1; then
  echo "git-gist: gg is already an alias; run 'unalias gg' and remove or rename it in your shell startup file." >&2
  return 1 2>/dev/null || exit 1
fi
case "$(command -V gg 2>/dev/null)" in
  *function*)
    echo "git-gist: gg is already a function; remove or rename it in your shell startup file." >&2
    return 1 2>/dev/null || exit 1
    ;;
esac

gg_cd() {
  name="$1"
  if [ -z "$name" ]; then
    echo "usage: gg_cd <alias>" >&2
    return 1
  fi
  path=$(gg alias list 2>/dev/null | awk -v n="$name" -F'	' '$1==n {print $2; exit}')
  if [ -z "$path" ]; then
    echo "gg_cd: alias not found: $name" >&2
    return 1
  fi
  cd "$path" || return 1
}
