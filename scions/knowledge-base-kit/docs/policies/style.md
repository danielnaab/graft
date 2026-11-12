---
status: stable
---
# Style policy

Write so that a reader, human or agent, gets the point fast and acts on it correctly.

## Keep it short by saying each thing once

- Put each fact on one page, and link to it from the others.
- Cut repetition and filler, never a reason or a limit. A rule without its reason gets broken.
- Context size matters: LLMs read these docs too. Short means no repetition, not missing facts.

## Choose the form for the content

- Bullets for parallel items. Prose for reasoning: the "because" between facts is the content.
- Tables for facts with the same fields. Headings and short sections for navigation.
- Concrete and specific: a command, a path, an example. No vague claims without sources.

## Procedures

Based on [ASD-STE100](https://www.asd-ste100.org/) (Simplified Technical English):

- Numbered steps in the imperative, one action per step.
- A condition comes before its action: "If the repository is private, add …".
- Notes and reasons come after the steps, not inside them.
- Short sentences: up to 20 words in a step, 25 in a description. Active voice.

## Terms

- One term for one concept, and one concept for one term. Define each term once, where it is introduced.
- Prefer the plain word. Explain jargon at first use, or link to its definition.

## Document types and lengths

| Type ([Diátaxis](https://diataxis.fr/)) | In this kit | Length |
| --- | --- | --- |
| How-to: steps toward a goal | playbook | 50-100 lines |
| Reference: rules, options, formats | policy, tool docs | policies 20-60 lines; reference as long as what it describes |
| Explanation: why, trade-offs | strategy, decision | 20-80 lines |
| Tutorial: a first guided success | (optional) | as short as the task allows |

Keep the types apart: a how-to links to the reference and the explanation instead of containing them.

Related:
- [Linking policy](linking.md)
- [Provenance policy](provenance.md)
