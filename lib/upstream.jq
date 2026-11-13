# upstream.jq: jq definitions shared through `include "upstream";` (jq -L lib): the text helpers,
# and one input's upstream notes (common.sh's upstream_range JSON) as graft migrate and graft
# changelog print them. Provider text leaves only as a block quote.

def cap(n): if length <= n then . else (.[0:n+1] | sub("\n[^\n]*$"; ""))
  + "\n\n(cut at \(n) characters; the full text is upstream)" end;
# upstream text as plain lines: CR and CRLF become LF, and every other control character (ESC, BEL,
# NUL, NEL, the line/paragraph separators, bidi overrides) a ?, so nothing in it can end a quote,
# move the cursor, write the clipboard or hide text
def clean: gsub("\r\n?"; "\n") | gsub("[\u0001-\u0008\u000b-\u001f\u007f\u0085  ‪-‮⁦-⁩]"; "?") | gsub("\u0000"; "?");
def quote: clean | split("\n") | map("> " + .) | join("\n");
# a path or name as one printable line
def safe: tostring | gsub("[\u0000-\u001f\u007f]"; "?");
# show at most N of an array of lines, saying how many more
def most(n): if length > n then .[0:n] + ["... and \(length - n) more"] else . end;

# upstream_notes(where): the lines for one input: its heading, then its CHANGELOG.md sections
# (Upgrading subsections first) or its commits, quoted. WHERE names the other side for `elsewhere`.
def upstream_notes($where):
  "### \(.name | safe) \(.from | safe) -> \(.to | safe)",
  "",
  (if .status == "added" then "A new input; nothing to compare.", ""
   elif .status == "removed" then "Removed from flake.lock.", ""
   elif .status == "elsewhere" then "This branch points \(.name | safe) at a different source than \($where | safe) does; not fetched. Read the diff of flake.lock.", ""
   elif .status == "not-commit" then "Not a git commit; nothing to compare.", ""
   elif .status == "not-git" then "Not a git input; nothing to compare.", ""
   elif .status == "unfetchable" then "Couldn't fetch \(if .to_rev == "?" then .to else .to_rev[0:12] end | safe) from \(.url | safe).", ""
   elif .changelog != "" then
     (if .upgrading != "" then
        "Upgrading, from its CHANGELOG.md:", "", (.upgrading | cap(8000) | quote), "",
        (if .rest != "" then "The rest of its CHANGELOG.md for this range, without the Upgrading sections above:", "", (.rest | cap(4000) | quote), "" else empty end)
      else
        "From its CHANGELOG.md (no Upgrading section in this range):", "", (.changelog | cap(6000) | quote), ""
      end)
   elif .in_history then
     "No CHANGELOG.md section for this range; its commits:", "",
     (.commits | most(50) | map("- " + .) | if length == 0 then ["- (none)"] else . end | join("\n") | cap(6000) | quote), ""
   else "No CHANGELOG.md section for this range, and \(.from_rev[0:12]) is not in its history.", "" end),
  (if .compare and (.status == "fetched" or .status == "unfetchable") then "Compare: \(.compare | safe)", "" else empty end);
