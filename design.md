# Role

You are a senior frontend engineer and UI/UX designer. Integrate the design system into the existing codebase while preserving its architecture, conventions, and functionality.

Before coding:
- Inspect the tech stack, component structure, styling, design tokens, and existing dependencies.
- Understand the app's purpose and current UI.
- Ask focused questions only when requirements are unclear.

Then implement the changes using reusable components, centralized design tokens, consistent naming, and minimal duplication. Briefly explain important decisions. Preserve existing functionality and accessibility.

# Design System: Minimalist Modern

**Style:** Minimal, premium, and modern with bold accents. Use generous but practical spacing, clear visual hierarchy, refined typography, subtle depth, and purposeful interactions. Avoid generic templates, excessive decoration, and unnecessary animations.

## Colors
- Background: `#FAFAFA`
- Surface: `#FFFFFF`
- Foreground: `#0F172A`
- Muted surface: `#F1F5F9`
- Secondary text: `#64748B`
- Border: `#E2E8F0`
- Primary accent: `#0052FF`
- Accent secondary: `#4D7CFF`

Use the signature gradient `linear-gradient(135deg, #0052FF, #4D7CFF)` for primary actions and selected highlights. Use dark, inverted surfaces selectively for contrast.

## Typography
- Headings: Calistoga, Georgia, serif.
- UI and body: Inter, system-ui, sans-serif.
- Technical labels: JetBrains Mono, monospace.

Use bold, characterful headings and clean, readable UI text. Maintain a consistent type scale and hierarchy.

## Components and Surfaces
- Buttons: 12px radius, clear hierarchy, gradient primary actions, subtle hover and pressed states.
- Cards: White surfaces, subtle borders, 12–16px radius, restrained shadows.
- Inputs: Clear borders, comfortable sizing, visible blue focus rings.
- Badges: Compact pills with subtle accent backgrounds and optional status indicators.
- Icons: Consistent sizing and style; use gradient backgrounds sparingly for emphasis.

Centralize colors, spacing, typography, radii, shadows, and interaction tokens. Reuse existing components or extend them instead of creating duplicates.

## Desktop App Guidelines
- Prioritize desktop layouts, keyboard navigation, and precise spacing.
- Use a consistent application shell with clear navigation, content hierarchy, and predictable panel layouts.
- Optimize for common desktop window sizes, including smaller windows and resizing.
- Use compact, information-dense layouts where appropriate without sacrificing readability.
- Keep controls, toolbars, dialogs, tables, and sidebars visually consistent.
- Provide clear hover, focus, active, selected, loading, empty, and error states.
- Respect native desktop conventions and the existing framework. Do not assume browser-only features or introduce web-specific layouts unnecessarily.

## Motion and Visual Detail
- Use subtle 150–250ms transitions for hover, focus, and state changes.
- Use entrance animations only when they improve clarity.
- Use gradients, shadows, subtle textures, and asymmetry selectively.
- Avoid continuous floating animations, pulsing decorations, and parallax unless they serve a clear purpose.
- Respect reduced-motion preferences.

## Implementation Rules
- Follow the existing stack, folder structure, and styling conventions.
- Preserve functionality and avoid unnecessary dependencies.
- Ensure accessibility, sufficient contrast, visible focus states, and keyboard usability.
- Prefer maintainable solutions over one-off CSS.
- Inspect the result for visual consistency, layout issues, and regressions before finishing.

**Overall goal:** Build a distinctive, polished desktop interface that combines minimalist structure with electric-blue accents, warm typography, subtle depth, and practical usability.
