# git-gist fish helpers
# Usage: source /path/to/shell/gg.fish

function gg-cd --description 'cd to a gg alias'
    if test (count $argv) -lt 1
        echo "usage: gg-cd <alias>" >&2
        return 1
    end
    set -l path (gg alias list 2>/dev/null | awk -v n="$argv[1]" -F'\t' '$1==n {print $2; exit}')
    if test -z "$path"
        echo "gg-cd: alias not found: $argv[1]" >&2
        return 1
    end
    cd $path
end

function gg-prompt --description 'dirty child repo count for prompt'
    set -l n (gg --only-dirty --color never list 2>/dev/null | wc -l | string trim)
    if test -n "$n"; and test "$n" != "0"
        echo "[gg:$n dirty]"
    end
end

if type -q gg
    gg completions fish 2>/dev/null | source
end
