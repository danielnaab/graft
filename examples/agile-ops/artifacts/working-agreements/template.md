# Team Working Agreements - Guidance

This template evaluates retrospective action items to help you update the team handbook.

---

## Recent Retrospective Decisions and Action Items

{% for material in materials %}
### From: {{ material.path | basename }}

**Source:** `{{ material.path }}`
**Last Modified:** {{ material.metadata.get('last_modified', 'N/A') }}

#### Decisions Made:
{% if '## Decisions Made' in material.content %}
{{ material.content.split('## Decisions Made')[1].split('##')[0] | trim }}
{% else %}
_No decisions section found_
{% endif %}

#### Action Items:
{% if '## Action Items' in material.content %}
{{ material.content.split('## Action Items')[1].split('##')[0] | trim }}
{% else %}
_No action items section found_
{% endif %}

---
{% endfor %}

## Guidance for Updating Team Handbook

Review the decisions and action items above. Update `team-handbook.md` to reflect:

1. **New working agreements** from retrospective decisions
2. **Process improvements** from action items
3. **Clarity on unclear procedures** that caused friction
4. **Examples** where helpful for new team members

Remember:
- Working agreements should be actionable, not aspirational
- Include rationale (the "why") from retrospectives
- Mark effective dates for new agreements
- Link back to retrospectives for context

After updating, finalize with team member attribution:
```
graft finalize artifacts/working-agreements/ --agent "Your Name" --role human
```

For significant changes, open a PR for team discussion and consensus.
