---
status: deprecated
---
# Change a grafted file

Moved to [Fix a provider from a consumer](fix-a-provider.md). Since v2.13.0 a scion is a git submodule, a
clone of the provider: you fix a provider in place and upstream the fix. To stop grafting a provider,
`git rm scions/NAME` ([Graft your first pattern](graft-a-repository.md#how-it-works)).
