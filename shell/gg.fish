# git-gist fish helpers
# Usage: source /path/to/shell/gg.fish

function __gg_has_top_level_abbr
    for definition in (abbr --show)
        printf '%s\n' "$definition" | read --tokenize --list words
        set -l separator (contains --index -- -- $words)
        set -q separator[1]; or continue
        set -l name "$words[(math "$separator + 1")]"
        set -l regex
        set -l commands
        set -l i 1
        while test $i -lt $separator
            set -l next (math "$i + 1")
            switch $words[$i]
                case --regex
                    set regex "$words[$next]"
                    set i (math "$i + 2")
                    continue
                case --command
                    set --append commands "$words[$next]"
                    set i (math "$i + 2")
                    continue
            end
            set i (math "$i + 1")
        end
        if set -q commands[1]; and not contains -- '' $commands
            continue
        end
        if set -q regex[1]
            string match --quiet --regex -- "^(?:$regex)\$" gg 2>/dev/null; and return 0
        else if test "$name" = gg
            return 0
        end
    end
    return 1
end

if functions -q gg
    echo "git-gist: gg is already a function; run 'functions -e gg' and remove or rename it in your shell startup file." >&2
    functions -e __gg_has_top_level_abbr
    return 1
else if __gg_has_top_level_abbr
    echo "git-gist: a Fish abbreviation expands top-level gg; remove or rename it in your shell startup file." >&2
    functions -e __gg_has_top_level_abbr
    return 1
end
functions -e __gg_has_top_level_abbr

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
    if not complete -c gg | string match -q '*'
        command gg completions fish 2>/dev/null | source
    end
end
