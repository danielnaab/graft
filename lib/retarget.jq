# retarget.jq: one input's url in a flake.nix moved from one tag to another. tools.sh runs it as
#   jq -Rsj -f retarget.jq --arg name NAME --arg type github|git --arg prefix P --arg old OLD --arg new NEW
# on the flake.nix text, and gets the new text, or (exit 1) one line on stderr saying why not.
#
# P is the input's url up to its ref, spelled as flake.lock's `original` records it:
# `github:OWNER/REPO` or `git+URL`. A match is P, after a quote, a space or the start of the text,
# then the rest of the url up to the first quote, space or `;`, which must be `/OLD` (github:
# only) or a query carrying `ref=OLD`, optionally after that `/OLD` for github:. So a url that
# merely starts with P (…/notes-old for …/notes) is another input's, and so is the same ref in
# another input's url. Exactly one match, or nothing
# changes: two inputs with the same url and ref can't be told apart from the text.

# the query Q with its ref=OLD parameter now ref=NEW, or null if it has none
def requery($q): ($q | split("&")) as $ps
  | if ($ps | index(["ref=" + $old])) == null then null
    else $ps | map(if . == "ref=" + $old then "ref=" + $new else . end) | join("&") end;

# REST, the url past P, retargeted; or null if it isn't this input at OLD
def swap($rest):
  ($rest | index("?")) as $q
  | (if $q == null then $rest else $rest[:$q] end) as $path
  | (if $q == null then null else $rest[$q + 1:] end) as $query
  | if $type == "github" and $path == "/" + $old then "/" + $new + (if $q == null then "" else "?" + $query end)
    elif $path == "" and $query != null then (requery($query) | if . == null then null else "?" + . end)
    else null end;

# split, length and slices all count codepoints, so text outside ASCII can't skew the offsets.
# Each part after the first starts just past one occurrence of P.
split($prefix) as $parts
| [ range(1; $parts | length) as $k
    # P starts the url: the text before it ends in a quote or a space, or is nothing
    | select(($k == 1 and $parts[0] == "") or ($parts[$k - 1] | test("[\"'\\s]$")))
    | ($parts[$k] | capture("^(?<r>[^\"'\\s;]*)").r) as $rest
    | swap($rest) as $to | select($to != null)
    | {k: $k, rest: $rest, to: $to} ] as $hits
| if ($hits | length) != 1 then
    "graft: \($name): expected its url (\($prefix) at \($old)) once in flake.nix, found it \($hits | length) times; nothing changed: change the ref by hand\n"
    | halt_error(1)
  else $hits[0] as $h
    | $parts | .[$h.k] = $h.to + (.[$h.k][$h.rest | length:]) | join($prefix) end
