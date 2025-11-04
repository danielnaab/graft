---
deps:
  - docs/logo-exploration/01-explorations/symbol-concepts.md
  - docs/logo-exploration/01-explorations/visual-metaphor-studies.md
  - docs/logo-exploration/01-explorations/color-palette.md
  - docs/logo-exploration/00-sources/technical-requirements.md
---
# Composition and Layout Studies

Refine the top 5-6 most promising symbol concepts from previous explorations by developing detailed compositional studies.

For each refined concept, provide:

## 1. Concept Name
Clear identifier from previous explorations (e.g., "Grafting Junction V3", "Stylized G with Branch")

## 2. Refined Visual Description
Detailed geometric description:
- **Core shapes**: Circles, rectangles, paths, and their dimensions
- **Proportions**: Relationships between elements (golden ratio, grid-based, etc.)
- **Balance**: Symmetry vs. asymmetry, visual weight distribution
- **Negative space**: What the empty spaces create
- **Optical adjustments**: Any visual corrections for better appearance

## 3. SVG Construction Outline
High-level outline of how this would be constructed in SVG:
- Viewbox dimensions (e.g., 0 0 100 100)
- Major elements and their approximate coordinates
- Shape primitives to use (circle, rect, path, etc.)
- Grouping strategy

Example:
```
viewBox: 0 0 100 100
- <circle> at (50, 35), r=25 for top element
- <path> diagonal from (25, 50) to (75, 50) for junction
- <circle> at (50, 75), r=20 for base element
```

## 4. Size Variations
How does this composition adapt at different sizes?
- **16x16px**: What simplifications needed? Any elements removed?
- **32x32px**: Standard icon size appearance
- **48x48px and up**: Full detail version

## 5. Color Application
Using palettes from color-palette exploration:
- Which palette(s) work best with this composition?
- How are colors distributed across elements?
- Does it work in monochrome?
- How does it reverse for dark mode?

## 6. Lockup Variations (if applicable)
If this is part of a logo lockup with text:
- **Icon only**: Standalone symbol
- **Horizontal lockup**: Symbol + "Graft" text side-by-side
- **Stacked lockup**: Symbol above text
- **Spacing and proportions**: Relationships between elements

## 7. Refinement Notes
What makes this composition successful:
- **Strengths**: What works exceptionally well?
- **Unique qualities**: What makes it distinctive?
- **Potential improvements**: What could be refined further?
- **Technical execution**: Any challenges in SVG implementation?

## Guidelines

- Focus on geometric precision and clean construction
- Consider optical adjustments (e.g., slightly larger top elements for visual balance)
- Think about how shapes read at extreme sizes (16px and 512px)
- Describe clearly enough that an SVG developer could implement it
- Note any mathematical relationships (golden ratio, fibonacci, grid-based)
- Consider how composition guides the eye
- Think about cultural/directional reading (left-to-right, top-to-bottom)

## Selection Criteria

Choose concepts to refine based on:
- High scores in symbol-concepts evaluation
- Strong metaphor connection from visual-metaphor-studies
- Technical feasibility for SVG implementation
- Diversity of approaches (don't refine 5 variations of the same idea)
- Potential for creating a complete visual system (icon + lockups + variants)

Aim for 5-6 refined compositions that represent the best thinking across all metaphor directions.
