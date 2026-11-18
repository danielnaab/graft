# graft.apis: the APIs an agent kit can be pointed at, as values (architecture §4, workflow 5):
#
#   graft.kits.claude-code { …; api = graft.apis.bedrock { region = "us-east-1"; }; }
#   graft.kits.claude-code { …; api = graft.apis.gateway { url = "https://llm.example.org"; }; }
#
# A value is data the kit reads: { name; region?; url?; host?; }. A kit decides what each means (hosts,
# credentials). A gateway (v2.17.1) is an Anthropic-compatible service at an https url, reached on port
# 443: its url without a trailing slash or :443, and its host, lowercased.
let
  gateway = { url }:
    let
      m = builtins.match "https://([A-Za-z0-9.-]+)(:443)?(/[A-Za-z0-9._~/%-]*)?" url;
      path = let p = builtins.elemAt m 2; in if p == null then "" else p;
      trimmed = let t = builtins.match "(.*[^/])/*" path; in if t == null then "" else builtins.head t;
      upper = [ "A" "B" "C" "D" "E" "F" "G" "H" "I" "J" "K" "L" "M" "N" "O" "P" "Q" "R" "S" "T" "U" "V" "W" "X" "Y" "Z" ];
      lower = [ "a" "b" "c" "d" "e" "f" "g" "h" "i" "j" "k" "l" "m" "n" "o" "p" "q" "r" "s" "t" "u" "v" "w" "x" "y" "z" ];
      host = builtins.replaceStrings upper lower (builtins.head m);
    in
    if m == null
    then throw "graft.apis.gateway: url = \"${url}\": give https://HOST[/PATH] (pot reaches HOST on port 443)"
    else { name = "gateway"; inherit host; url = "https://${host}${trimmed}"; };
in
{
  anthropic = { name = "anthropic"; };
  bedrock = { region }: { name = "bedrock"; inherit region; };
  inherit gateway;
}
