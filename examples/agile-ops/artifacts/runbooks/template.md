# On-Call Runbook - Update Guidance

This template evaluates incident post-mortems and working agreements to help you update the on-call runbook.

---

## Working Agreements: On-Call Section

{% for material in materials %}
{% if 'working-agreements' in material.path or 'team-handbook' in material.path %}
**Source:** `{{ material.path }}`

### On-Call Agreement:
{% if '## On-Call' in material.content %}
{{ material.content.split('## On-Call')[1].split('##')[0] | trim }}
{% else %}
_No on-call section found in working agreements_
{% endif %}
{% endif %}
{% endfor %}

---

## Recent Incidents and Lessons Learned

{% for material in materials %}
{% if 'incidents' in material.path %}
### {{ material.path | basename | replace('.md', '') }}

**Source:** `{{ material.path }}`

#### Summary:
{% if '## Summary' in material.content %}
{{ material.content.split('## Summary')[1].split('##')[0] | trim }}
{% endif %}

#### What Went Wrong:
{% if '## What Went Wrong' in material.content %}
{{ material.content.split('## What Went Wrong')[1].split('##')[0] | trim }}
{% endif %}

#### Action Items:
{% if '## Action Items' in material.content %}
{{ material.content.split('## Action Items')[1].split('##')[0] | trim }}
{% endif %}

#### Lessons Learned:
{% if '## Lessons Learned' in material.content %}
{{ material.content.split('## Lessons Learned')[1].split('##')[0] | trim }}
{% endif %}

---
{% endif %}
{% endfor %}

---

## Guidance for Updating Runbook

Review the incidents and action items above. Update `on-call-runbook.md` to reflect:

1. **New escalation procedures** identified in incidents
2. **Deployment rollback steps** documented from experience
3. **Common failure modes** and their resolutions
4. **Contact information** that was missing or unclear
5. **Decision criteria** for when to rollback vs. debug
6. **Health check procedures** for post-deploy verification

**Best Practice:** Update runbooks immediately after incidents while details are fresh.

Remember:
- Runbooks should be actionable step-by-step procedures, not conceptual docs
- Include commands, not just descriptions
- Add decision trees for common scenarios
- Keep contact information current
- Link to related incidents for context

After updating, finalize with your attribution:
```
graft finalize artifacts/runbooks/ --agent "Your Name" --role human
```

Any team member can update runbooks—shared ownership means shared maintenance.
