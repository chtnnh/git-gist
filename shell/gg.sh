# git-gist POSIX sh helpers (minimal)
# Usage: . /path/to/shell/gg.sh

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
