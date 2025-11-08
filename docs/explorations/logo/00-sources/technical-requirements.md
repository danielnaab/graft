# Technical Requirements for Graft Logo

## SVG Format Specifications

### File Format
- **Primary format**: SVG (Scalable Vector Graphics)
- **SVG version**: SVG 1.1 (maximum compatibility)
- **Encoding**: UTF-8
- **Standalone**: No external dependencies (embedded fonts, external images)

### Code Quality Standards
- Clean, readable SVG code
- Minimal use of transforms (prefer direct coordinates)
- Use `<path>` elements for complex shapes
- Use `<circle>`, `<rect>`, `<line>` for simple geometry
- Group related elements with `<g>` tags
- Include meaningful `id` attributes for major elements

### File Size
- **Target**: < 2KB for simple logos
- **Maximum**: < 5KB including all variants
- **Optimization**: Remove unnecessary precision (2 decimal places max)
- **Compression**: Should compress well with gzip

### Example SVG Structure
```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">
  <title>Graft Logo</title>
  <desc>Logo for Graft documentation orchestration system</desc>
  <g id="logo-primary">
    <!-- Logo elements here -->
  </g>
</svg>
```

## Size and Scale Requirements

### Minimum Sizes (Must Be Legible)
- **16x16px**: Favicon, smallest icon size
- **32x32px**: Standard icon size
- **48x48px**: App icon, large favicon
- **64x64px**: High-DPI icon

### Optimal Display Sizes
- **128x128px**: Social media profile
- **200x200px**: GitHub README inline
- **256x256px**: Documentation headers
- **512x512px**: Large presentations, hero images

### Aspect Ratio
- **Preferred**: Square (1:1) for maximum versatility
- **Alternative**: Horizontal rectangle (3:2 or 16:9) for lockups with text
- **ViewBox**: Should use consistent coordinate system (e.g., 0 0 100 100)

### Stroke Widths
At different scales, stroke widths should:
- **16px**: Minimum 1.5-2px stroke (15-20% of size)
- **32px**: Minimum 1-1.5px stroke (3-5% of size)
- **100px+**: Can use finer details, but not required

Design should not rely on strokes thinner than 1px at reference size.

## Color Specifications

### Color Mode
- **Primary**: RGB (for screens)
- **Color space**: sRGB
- **Format**: Hex codes (#RRGGBB) or named colors

### Color Variants Required

#### 1. Full Color Version
- **Maximum 3 colors** (including any grayscale)
- Each color must be defined with hex code
- Include both light and dark mode variants if needed

#### 2. Monochrome Version
- **Single color**: Usually black (#000000) or brand primary
- Must be recognizable in monochrome
- Should work inverted (white on dark)

#### 3. Reversed Version
- Light colors for dark backgrounds
- Typically white (#FFFFFF) or very light tones
- Should maintain contrast ratios

### Accessibility Requirements
- **Contrast ratio**: Minimum 3:1 against background (for non-text graphics)
- **Color blindness**: Should be distinguishable in major color blindness types
- **Monochrome fallback**: Must work without color information

## Embedding Specifications

### Markdown/HTML Embedding
Logo should be embeddable in markdown with simple syntax:

```markdown
![Graft Logo](docs/logo.svg)
```

Or inline HTML:
```html
<img src="docs/logo.svg" alt="Graft Logo" width="200">
```

### Inline SVG
Should work when embedded directly in HTML:
```html
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <!-- Logo SVG code -->
</svg>
```

### Styling Hooks
- Include CSS classes or IDs for styling
- Support for `fill` and `stroke` CSS properties
- Allow size scaling via `width` and `height` attributes

## Theme Compatibility

### Light Mode
- **Background**: White (#FFFFFF) or light gray (#F5F5F5)
- **Logo colors**: Should have sufficient contrast
- **Primary variant**: Designed for light backgrounds

### Dark Mode
- **Background**: Dark gray (#1E1E1E) or black (#000000)
- **Logo colors**: Should be visible and readable
- **Alternative variant**: May need lightened or reversed colors

### Implementation Options

**Option 1: CSS Variables**
```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <style>
    :root { --logo-color: #2C5F2D; }
    @media (prefers-color-scheme: dark) {
      :root { --logo-color: #97BC62; }
    }
  </style>
  <path fill="var(--logo-color)" d="..."/>
</svg>
```

**Option 2: Separate Files**
- `logo-light.svg` - Optimized for light backgrounds
- `logo-dark.svg` - Optimized for dark backgrounds

## File Organization

### Recommended File Structure
```
docs/
├── logo.svg                    # Primary full-color logo
├── logo-monochrome.svg         # Single color version
├── logo-reversed.svg           # Light version for dark backgrounds
├── logo-icon.svg               # Icon only (no text if lockup)
└── variants/
    ├── logo-16.svg             # Optimized for 16px
    ├── logo-32.svg             # Optimized for 32px
    └── logo-favicon.ico        # Converted to ICO format
```

### Exports Needed
- **SVG**: Primary, editable, scalable
- **PNG**: Rasterized for specific contexts (16, 32, 64, 128, 256, 512px)
- **ICO**: Favicon format (optional, can be generated from PNG)

## Performance Considerations

### Load Time
- SVG should load instantly (< 5KB)
- No external dependencies to fetch
- No complex filters or effects requiring processing

### Rendering Performance
- Simple geometry renders faster
- Avoid excessive path points
- Minimize use of gradients, filters, masks
- Test rendering performance at 60fps animation (if animated variant)

### Browser Compatibility
- Must work in all modern browsers (Chrome, Firefox, Safari, Edge)
- Graceful fallback for older browsers (IE11 not required)
- No browser-specific SVG features

## Geometric Precision

### Grid System
- Design on a consistent grid (e.g., 100x100 unit canvas)
- Use round numbers for coordinates when possible
- Align elements to grid for pixel-perfect rendering

### Coordinate Precision
- Maximum 2 decimal places (e.g., 23.45 not 23.456789)
- Remove trailing zeros
- Simplify paths where possible

### Bezier Curves
- Use smooth curves (avoid jaggy paths)
- Minimize number of control points
- Ensure curves render well when scaled

## Testing Requirements

Logo must be tested at:

### Different Sizes
- Render at 16, 32, 48, 64, 128, 256px
- Verify legibility at each size
- Check stroke weights remain visible

### Different Backgrounds
- White background
- Light gray background (#F5F5F5)
- Dark gray background (#1E1E1E)
- Black background
- Brand color backgrounds (if applicable)

### Different Contexts
- GitHub README (light and dark mode)
- Browser favicon (multiple sizes)
- IDE/editor sidebar
- Documentation site header
- Social media profile picture (circular crop)

### Accessibility Testing
- Grayscale conversion (remove all color)
- Color blindness simulation (deuteranopia, protanopia, tritanopia)
- High contrast mode
- Screen reader compatibility (proper alt text)

## README Embedding Example

The final logo should work perfectly in this context:

```markdown
# Graft

![Graft Logo](docs/logo.svg)

Git-native documentation orchestration system.

[Rest of README...]
```

Goals for README display:
- Visually appealing at ~200px width
- Professional and polished
- Reinforces brand name
- Doesn't overwhelm the page
- Renders quickly and correctly

## Deliverables Checklist

For the final logo recommendation, provide:

- [ ] Primary SVG file (full color)
- [ ] Monochrome SVG variant
- [ ] Reversed SVG variant (for dark backgrounds)
- [ ] Icon-only variant (if applicable)
- [ ] Usage guidelines (when to use which variant)
- [ ] Color specifications (hex codes)
- [ ] Minimum size recommendations
- [ ] Code snippet for README embedding
- [ ] Visual examples at multiple sizes

## Quality Assurance

Before finalizing, verify:
- [ ] Valid SVG markup (no errors)
- [ ] Renders correctly in all major browsers
- [ ] File size within limits (< 5KB)
- [ ] Legible at 16x16px
- [ ] Works in monochrome
- [ ] Proper contrast ratios
- [ ] Clean, readable code
- [ ] Includes proper metadata (title, desc)
- [ ] No accessibility barriers
- [ ] Scales smoothly at all sizes
