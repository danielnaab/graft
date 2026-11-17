# graft.apis: the APIs an agent kit can be pointed at, as values (architecture §4, workflow 5):
#
#   graft.kits.claude-code { …; api = graft.apis.bedrock { region = "us-east-1"; }; }
#
# A value is data the kit reads: { name; region?; }. A kit decides what each means (hosts, credentials).
{
  anthropic = { name = "anthropic"; };
  bedrock = { region }: { name = "bedrock"; inherit region; };
}
