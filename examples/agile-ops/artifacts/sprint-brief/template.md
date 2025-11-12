# Sprint Brief - Template

This template pulls from the product roadmap and current backlog to guide sprint planning.

---

## Product Roadmap Context

{% for material in materials %}
{% if 'roadmap' in material.path %}
**Source:** `{{ material.path }}`

### Current Quarter Strategic Goals:
{% if '## Strategic Goals' in material.content %}
{{ material.content.split('## Strategic Goals')[1].split('##')[0] | trim }}
{% endif %}

### Current Sprint Context:
{% if '### Sprint' in material.content %}
{# Extract the current sprint section - this is simplified, real template would be smarter #}
_Review the roadmap for current sprint priorities and themes_
{% endif %}
{% endif %}
{% endfor %}

---

## Backlog Items for Sprint

{% for material in materials %}
{% if 'backlog' in material.path %}
**Source:** `{{ material.path }}`

_Include backlog items selected for this sprint during planning_
{% endif %}
{% endfor %}

---

## Guidance for Sprint Brief

Use this template to create your sprint brief during planning:

1. **Review roadmap themes** - What strategic goals are we working towards?
2. **Select backlog items** - What stories/tasks support those goals?
3. **Identify dependencies** - What blocks us or needs coordination?
4. **Set sprint goals** - What do we commit to completing?
5. **Note risks** - What might prevent us from achieving goals?

After sprint planning, update `brief.md` with:
- Sprint goals and commitments
- Key stories and point estimates
- Dependencies and risks
- Success criteria

Then finalize:
```
graft finalize artifacts/sprint-brief/ --agent "PM Name" --role human
```
