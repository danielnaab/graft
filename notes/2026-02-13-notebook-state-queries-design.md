---
status: analysis
date: 2026-02-13
context: Designing state queries for notebook/notes vault repository
---

# Notebook State Queries - Design Analysis

## Context

Designing state queries for a notes vault repository to expose useful information in Grove's repository detail pane, enabling more effective note-taking workflows.

## Core Use Cases for Notes Vault

### 1. Daily Writing Practice
- Track writing momentum (streak tracking)
- See today's progress (word count, notes created)
- Identify gaps (days without notes)
- Monitor writing velocity over time

### 2. Task/TODO Management
- Track open tasks across all notes
- See overdue tasks
- Monitor task completion rate
- Identify task hotspots (which notes have most tasks)

### 3. Knowledge Graph Health
- Detect orphaned notes (no backlinks)
- Find broken links
- Identify hub notes (highly connected)
- Track graph density (connectedness)

### 4. Content Discovery
- Find recently modified notes
- See notes created this week
- Identify stale notes (not touched in months)
- Track notes by tag/category

### 5. Quality & Maintenance
- Detect empty or stub notes
- Find duplicate titles
- Check for formatting issues
- Identify notes without frontmatter

---

## Proposed State Queries

### Category 1: Writing Metrics

#### `writing-today`
**Purpose**: Daily writing accountability
**Output**:
```json
{
  "date": "2026-02-13",
  "notes_created": 3,
  "notes_modified": 7,
  "words_added": 1247,
  "total_words": 45823,
  "streak_days": 12
}
```

**Implementation**:
```yaml
state:
  writing-today:
    run: |
      python3 << 'EOF'
      import json
      from datetime import datetime, timedelta
      from pathlib import Path
      import re

      def count_words(text):
          return len(re.findall(r'\w+', text))

      def parse_date_from_frontmatter(text):
          # Extract date from YAML frontmatter
          match = re.search(r'date:\s*(\d{4}-\d{2}-\d{2})', text)
          return match.group(1) if match else None

      today = datetime.now().date()
      notes_dir = Path('.')

      notes_created_today = 0
      notes_modified_today = 0
      words_added_today = 0
      total_words = 0

      for note_path in notes_dir.rglob('*.md'):
          if note_path.is_file() and not str(note_path).startswith('.'):
              text = note_path.read_text()
              total_words += count_words(text)

              # Check creation date
              note_date = parse_date_from_frontmatter(text)
              if note_date and note_date == str(today):
                  notes_created_today += 1
                  words_added_today += count_words(text)

              # Check modification time
              mtime = datetime.fromtimestamp(note_path.stat().st_mtime).date()
              if mtime == today:
                  notes_modified_today += 1

      # Calculate streak (simplified - could check git history)
      streak_days = 0
      check_date = today
      for i in range(365):
          has_activity = any(
              datetime.fromtimestamp(p.stat().st_mtime).date() == check_date
              for p in notes_dir.rglob('*.md')
              if p.is_file()
          )
          if has_activity:
              streak_days += 1
              check_date -= timedelta(days=1)
          else:
              break

      print(json.dumps({
          "date": str(today),
          "notes_created": notes_created_today,
          "notes_modified": notes_modified_today,
          "words_added": words_added_today,
          "total_words": total_words,
          "streak_days": streak_days
      }))
      EOF
    cache:
      deterministic: false  # Changes throughout the day
    timeout: 30
```

**Grove Display**:
```
Writing Today
  ✍️  3 notes created, 7 modified
  📝 1,247 words added (45,823 total)
  🔥 12-day streak
```

---

#### `writing-weekly`
**Purpose**: Weekly writing summary
**Output**:
```json
{
  "week_start": "2026-02-09",
  "notes_created": 18,
  "notes_modified": 42,
  "words_added": 8432,
  "most_active_day": "Tuesday",
  "avg_words_per_day": 1204
}
```

---

### Category 2: Task Management

#### `tasks-overview`
**Purpose**: Quick view of task status
**Output**:
```json
{
  "total_tasks": 47,
  "open_tasks": 23,
  "completed_today": 5,
  "overdue_tasks": 3,
  "by_priority": {
    "high": 8,
    "medium": 12,
    "low": 3
  },
  "top_notes_with_tasks": [
    {"note": "projects/website-redesign.md", "open": 12},
    {"note": "daily/2026-02-13.md", "open": 5}
  ]
}
```

**Implementation**:
```yaml
state:
  tasks-overview:
    run: |
      python3 << 'EOF'
      import json
      import re
      from pathlib import Path
      from datetime import datetime, date

      def parse_task(line):
          # Match: - [ ] Task text [priority: high] [due: 2026-02-15]
          task_match = re.match(r'-\s+\[([x\s])\]\s+(.+)', line)
          if not task_match:
              return None

          completed = task_match.group(1).lower() == 'x'
          text = task_match.group(2)

          priority_match = re.search(r'\[priority:\s*(\w+)\]', text)
          priority = priority_match.group(1) if priority_match else 'medium'

          due_match = re.search(r'\[due:\s*(\d{4}-\d{2}-\d{2})\]', text)
          due = due_match.group(1) if due_match else None

          return {
              'completed': completed,
              'text': text,
              'priority': priority,
              'due': due
          }

      notes_dir = Path('.')
      today = date.today()

      all_tasks = []
      tasks_by_note = {}

      for note_path in notes_dir.rglob('*.md'):
          if not note_path.is_file() or str(note_path).startswith('.'):
              continue

          note_tasks = []
          for line in note_path.read_text().splitlines():
              task = parse_task(line)
              if task:
                  task['note'] = str(note_path)
                  all_tasks.append(task)
                  if not task['completed']:
                      note_tasks.append(task)

          if note_tasks:
              tasks_by_note[str(note_path)] = len(note_tasks)

      open_tasks = [t for t in all_tasks if not t['completed']]
      completed_tasks = [t for t in all_tasks if t['completed']]

      # Count completed today (check git log in real implementation)
      completed_today = len([t for t in completed_tasks if True])  # Simplified

      # Count overdue
      overdue = [
          t for t in open_tasks
          if t['due'] and datetime.strptime(t['due'], '%Y-%m-%d').date() < today
      ]

      # Priority breakdown
      by_priority = {}
      for task in open_tasks:
          p = task['priority']
          by_priority[p] = by_priority.get(p, 0) + 1

      # Top notes with tasks
      top_notes = sorted(
          [{'note': n, 'open': c} for n, c in tasks_by_note.items()],
          key=lambda x: x['open'],
          reverse=True
      )[:5]

      print(json.dumps({
          'total_tasks': len(all_tasks),
          'open_tasks': len(open_tasks),
          'completed_today': completed_today,
          'overdue_tasks': len(overdue),
          'by_priority': by_priority,
          'top_notes_with_tasks': top_notes
      }))
      EOF
    cache:
      deterministic: true  # Tied to git commit
    timeout: 30
```

**Grove Display**:
```
Tasks
  📋 23 open, 47 total
  ✅ 5 completed today
  ⚠️  3 overdue
  Priority: 8 high, 12 medium, 3 low
```

---

### Category 3: Knowledge Graph

#### `graph-health`
**Purpose**: Assess knowledge graph connectivity
**Output**:
```json
{
  "total_notes": 234,
  "total_links": 456,
  "orphaned_notes": 12,
  "broken_links": 3,
  "hub_notes": [
    {"note": "index.md", "backlinks": 45},
    {"note": "concepts/meta-learning.md", "backlinks": 23}
  ],
  "avg_links_per_note": 1.95,
  "graph_density": 0.016
}
```

**Implementation**:
```yaml
state:
  graph-health:
    run: |
      python3 << 'EOF'
      import json
      import re
      from pathlib import Path
      from collections import defaultdict

      def extract_wikilinks(text):
          # Match [[link]] or [[link|display]]
          return re.findall(r'\[\[([^\]|]+)(?:\|[^\]]+)?\]\]', text)

      notes_dir = Path('.')

      # Build note index
      notes = {}
      for note_path in notes_dir.rglob('*.md'):
          if note_path.is_file() and not str(note_path).startswith('.'):
              notes[str(note_path)] = note_path.read_text()

      # Track links and backlinks
      outbound_links = defaultdict(list)
      inbound_links = defaultdict(list)
      broken_links = []

      for note_path, content in notes.items():
          links = extract_wikilinks(content)
          for link in links:
              # Resolve link (simplified - assumes same directory)
              target = None
              for candidate in notes.keys():
                  if link in candidate or link + '.md' in candidate:
                      target = candidate
                      break

              if target:
                  outbound_links[note_path].append(target)
                  inbound_links[target].append(note_path)
              else:
                  broken_links.append({'source': note_path, 'link': link})

      # Find orphaned notes (no inbound links)
      orphaned = [n for n in notes.keys() if not inbound_links[n]]

      # Find hub notes (many backlinks)
      hubs = sorted(
          [{'note': n, 'backlinks': len(links)} for n, links in inbound_links.items()],
          key=lambda x: x['backlinks'],
          reverse=True
      )[:10]

      # Calculate graph metrics
      total_links = sum(len(links) for links in outbound_links.values())
      avg_links = total_links / len(notes) if notes else 0
      max_possible_links = len(notes) * (len(notes) - 1)
      density = total_links / max_possible_links if max_possible_links > 0 else 0

      print(json.dumps({
          'total_notes': len(notes),
          'total_links': total_links,
          'orphaned_notes': len(orphaned),
          'broken_links': len(broken_links),
          'hub_notes': hubs[:5],
          'avg_links_per_note': round(avg_links, 2),
          'graph_density': round(density, 3)
      }))
      EOF
    cache:
      deterministic: true
    timeout: 60
```

**Grove Display**:
```
Knowledge Graph
  📊 234 notes, 456 links
  🔗 Avg 1.95 links/note (density: 0.016)
  ⚠️  12 orphaned, 3 broken
  🌟 Top: index.md (45 backlinks)
```

---

### Category 4: Content Discovery

#### `recent-activity`
**Purpose**: What's been happening lately
**Output**:
```json
{
  "last_modified": {
    "note": "projects/graft-state-queries.md",
    "timestamp": "2026-02-13T15:32:00Z",
    "minutes_ago": 5
  },
  "modified_today": [
    {"note": "daily/2026-02-13.md", "changes": 127},
    {"note": "projects/notebook-design.md", "changes": 43}
  ],
  "created_this_week": [
    {"note": "concepts/state-queries.md", "date": "2026-02-12"},
    {"note": "daily/2026-02-11.md", "date": "2026-02-11"}
  ],
  "stale_notes": 23
}
```

**Implementation**:
```yaml
state:
  recent-activity:
    run: |
      python3 << 'EOF'
      import json
      from pathlib import Path
      from datetime import datetime, timedelta

      notes_dir = Path('.')
      now = datetime.now()
      today = now.date()
      week_ago = today - timedelta(days=7)

      notes_by_mtime = []
      modified_today = []
      created_this_week = []
      stale_count = 0

      for note_path in notes_dir.rglob('*.md'):
          if not note_path.is_file() or str(note_path).startswith('.'):
              continue

          stat = note_path.stat()
          mtime = datetime.fromtimestamp(stat.st_mtime)
          ctime = datetime.fromtimestamp(stat.st_ctime)

          notes_by_mtime.append({
              'note': str(note_path),
              'mtime': mtime
          })

          # Modified today
          if mtime.date() == today:
              modified_today.append({
                  'note': str(note_path),
                  'changes': stat.st_size  # Simplified - could use git diff
              })

          # Created this week
          if ctime.date() >= week_ago:
              created_this_week.append({
                  'note': str(note_path),
                  'date': str(ctime.date())
              })

          # Stale (not modified in 90 days)
          if (now - mtime).days > 90:
              stale_count += 1

      # Most recent
      notes_by_mtime.sort(key=lambda x: x['mtime'], reverse=True)
      last_modified = notes_by_mtime[0] if notes_by_mtime else None

      if last_modified:
          minutes_ago = int((now - last_modified['mtime']).total_seconds() / 60)
          last_modified = {
              'note': last_modified['note'],
              'timestamp': last_modified['mtime'].isoformat(),
              'minutes_ago': minutes_ago
          }

      print(json.dumps({
          'last_modified': last_modified,
          'modified_today': modified_today[:10],
          'created_this_week': created_this_week,
          'stale_notes': stale_count
      }))
      EOF
    cache:
      deterministic: false  # Changes frequently
    timeout: 30
```

**Grove Display**:
```
Recent Activity
  🕐 Last: graft-state-queries.md (5m ago)
  📝 7 notes modified today
  🆕 3 created this week
  💤 23 stale notes (>90 days)
```

---

### Category 5: Tags & Organization

#### `tags-overview`
**Purpose**: Content organization by tags
**Output**:
```json
{
  "total_tags": 42,
  "notes_with_tags": 187,
  "notes_without_tags": 47,
  "top_tags": [
    {"tag": "project", "count": 34},
    {"tag": "meeting", "count": 28},
    {"tag": "idea", "count": 23}
  ],
  "new_tags_this_week": ["state-queries", "grove-design"]
}
```

---

## Recommended Initial Set

For a **minimal viable state tracking setup**, start with these 4 queries:

1. **`writing-today`** - Daily motivation and accountability
2. **`tasks-overview`** - Task management at a glance
3. **`graph-health`** - Content connectivity health
4. **`recent-activity`** - Quick discovery of recent work

These cover the most common workflows and provide immediate value in Grove.

---

## Grove Integration Benefits

### 1. At-a-Glance Status
When opening Grove, immediately see:
- Writing streak status → Motivation to maintain momentum
- Open tasks count → Reminder of commitments
- Recent activity → Easy pickup from where you left off
- Graph health → Identify maintenance needs

### 2. Workflow Enablement
State queries enable smart workflows:
- **Daily Review**: Check `writing-today`, see if you hit your word count goal
- **Weekly Planning**: Review `tasks-overview`, prioritize week ahead
- **Maintenance Days**: Use `graph-health` to find orphaned notes, broken links
- **Content Discovery**: Browse `recent-activity` to surface forgotten notes

### 3. Historical Analysis
With temporal queries (`--commit HEAD~30`):
- Track writing velocity over time
- See how task load evolves
- Watch knowledge graph grow
- Identify productivity patterns

---

## Implementation Strategy

### Phase 1: Core Metrics (1-2 hours)
1. Implement `writing-today` query
2. Test in Grove detail pane
3. Verify caching behavior
4. Iterate on display format

### Phase 2: Task Management (2-3 hours)
1. Implement `tasks-overview` query
2. Define task syntax conventions
3. Add parsing logic
4. Test with real notes

### Phase 3: Graph Analysis (3-4 hours)
1. Implement `graph-health` query
2. Add wikilink parsing
3. Build backlink index
4. Calculate graph metrics

### Phase 4: Refinement (1-2 hours)
1. Add `recent-activity` query
2. Optimize query performance
3. Improve Grove display formatting
4. Document conventions in notebook README

---

## Technical Considerations

### Performance
- **Query Execution Time**: Target < 5s for 1000 notes
- **Caching Strategy**: Deterministic queries cache by commit, non-deterministic queries run fresh
- **Incremental Processing**: For large vaults (10k+ notes), consider indexing approach

### Conventions
Define clear conventions in notebook repository:
- **Task Syntax**: `- [ ] Task [priority: high] [due: YYYY-MM-DD]`
- **Tags**: `tags: [project, important]` in frontmatter
- **Wikilinks**: `[[note-name]]` or `[[note-name|display]]`
- **Date Format**: ISO 8601 (`YYYY-MM-DD`) everywhere

### Error Handling
- **Missing Dependencies**: Check for `python3`, `jq` availability
- **Malformed Notes**: Handle notes without frontmatter gracefully
- **Empty Vault**: Return zero values, not errors

---

## Example Grove Display (Combined)

```
notebook repository (~/Documents/notebook)
├─ Status: Clean, 3 files modified
├─ Branch: main (up to date)
├─ Latest: "Add state queries design" (2 minutes ago)
│
├─ Writing Today
│  ✍️  3 notes created, 7 modified
│  📝 1,247 words added (45,823 total)
│  🔥 12-day streak
│
├─ Tasks
│  📋 23 open, 47 total
│  ✅ 5 completed today
│  ⚠️  3 overdue
│  Priority: 8 high, 12 medium, 3 low
│
├─ Knowledge Graph
│  📊 234 notes, 456 links
│  🔗 Avg 1.95 links/note (density: 0.016)
│  ⚠️  12 orphaned, 3 broken
│  🌟 Top: index.md (45 backlinks)
│
└─ Recent Activity
   🕐 Last: graft-state-queries.md (5m ago)
   📝 7 notes modified today
   🆕 3 created this week
   💤 23 stale notes (>90 days)
```

---

## Next Steps

1. **Validate Use Cases**: Confirm these metrics align with your actual notebook workflows
2. **Start Small**: Implement `writing-today` first, iterate on format
3. **Define Conventions**: Document task syntax, tag format, etc. in notebook README
4. **Test Performance**: Verify query speed on your actual vault size
5. **Refine Display**: Work with Grove to optimize how state appears in detail pane

The key is starting simple and evolving based on what you actually find useful in practice.
