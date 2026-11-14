---
status: deprecated
---
# Ship a tool or seed files with a pattern

Moved to [Write a provider's graft.nix](write-graft-nix.md). Since v2.13.0, a provider ships tools, checks
and apps through a `graft.nix` at its top, and consumers no longer build them from a fetched export.
