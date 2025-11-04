---
deps:
  - docs/naming-exploration/02-final/recommendation.md
  - docs/getting-started.md
  - docs/use-cases.md
  - docs/how-it-works.md
  - docs/configuration.md
  - docs/command-reference.md
  - docs/project-overview.md
---

Generate a professional, high-level README.md file for the Graft project that follows modern open-source README best practices.

**IMPORTANT**: Output the README content directly as markdown. Do NOT wrap the output in code fences or any other container.

## Key Requirements

### Logo and Branding
- Start with the Graft logo centered using a `<picture>` element for dark mode support:
  ```html
  <div align="center">

  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/logo-dark.svg">
    <img src="docs/assets/logo.svg" alt="Graft Logo" width="200">
  </picture>

  </div>
  ```
- Center the logo and main heading in a clean, professional layout

### Project Description and Metaphor
The name "Graft" reflects the core functionality: like grafting branches in horticulture or commits in git, this tool takes content from multiple sources and carefully integrates them into a unified whole. The description should:
- Lead with a clear, concise one-liner explaining what Graft does
- Naturally incorporate the grafting metaphor (synthesis, integration, binding sources together)
- Emphasize the git-native approach and how Graft "grafts" documentation sources while preserving semantic intent
- Be professional and accessible without being overly technical in the introduction

### Structure
Follow this structure, keeping it CONCISE and HIGH-LEVEL:
1. **Header** - Logo, title, compelling tagline (centered)
2. **Overview** - What is Graft and why it exists (1-2 SHORT paragraphs incorporating the grafting metaphor)
3. **Key Features** - Bullet list of main capabilities (5-7 items max)
4. **Quick Start** - Basic installation and initialization only (3-4 commands max)
5. **Documentation** - Clear links to all detailed documentation:
   - Getting Started
   - Use Cases
   - How It Works
   - Configuration
   - Command Reference
   - Troubleshooting
6. **License** - CC0 1.0 Universal License statement

IMPORTANT: Keep the README scannable, compelling, and quick to read. Avoid detailed examples or usage patterns that could become outdated or inaccurate - link to documentation instead.

### Tone and Style
- Professional but approachable
- Clear and concise - FAVOR BREVITY
- Developer-focused - respect their time
- Use active voice and plain language
- Incorporate the grafting metaphor naturally in the overview without forcing it
- Avoid jargon or overly technical language in the high-level description

### Content Synthesis
Draw from the input documents to create a cohesive, high-level README that:
- Synthesizes key points from getting-started.md, how-it-works.md, and project-overview.md
- Links to detailed documentation rather than duplicating content
- Highlights what makes Graft unique (git-native, AI-powered, intelligent change detection)
- Makes the value proposition immediately clear
- Directs readers to the comprehensive documentation for details
- Avoids technical implementation details that belong in dedicated docs

The README should make a developer immediately understand what Graft does, why they'd want to use it, and where to learn more - all while subtly reinforcing the grafting metaphor of bringing multiple sources together into a unified whole.
