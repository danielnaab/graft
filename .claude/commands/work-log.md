---
description: Update today's work log with session progress
allowed-tools: Read(agent-records/work-log/**), Edit(agent-records/work-log/**), Write(agent-records/work-log/**), Bash(date:*)
---
Update the work log for today:

1. Determine today's date and construct path: `agent-records/work-log/YYYY-MM-DD.md`

2. If the file doesn't exist:
   - Create it with the header `# Work Log YYYY-MM-DD`
   - Add the first session section

3. If the file exists:
   - Read it to understand the current state
   - Add a new session section at the end

4. In the session section, document:
   - **Session N: [Brief Title]**
   - **Completed Tasks** (what was accomplished)
     - List specific files modified/created
     - Test results (pass/fail counts)
     - Bug fixes applied
     - Architecture decisions made
   - **Key Decisions** (important choices and rationale)
   - **Files Modified** (with paths)
   - **Files Created** (with paths)
   - **Next Steps** (what should be done next)
   - **Notes** (any additional context)

5. Follow the format from existing work logs in `agent-records/work-log/`

6. Be specific: include file paths, line numbers, test names, and technical details

Remember: Work logs are crucial for maintaining context across sessions and tracking project history.
