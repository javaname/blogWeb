---
name: Editorial Precision
colors:
  surface: '#f8f9fa'
  surface-dim: '#d9dadb'
  surface-bright: '#f8f9fa'
  surface-container-lowest: '#ffffff'
  surface-container-low: '#f3f4f5'
  surface-container: '#edeeef'
  surface-container-high: '#e7e8e9'
  surface-container-highest: '#e1e3e4'
  on-surface: '#191c1d'
  on-surface-variant: '#424754'
  inverse-surface: '#2e3132'
  inverse-on-surface: '#f0f1f2'
  outline: '#727785'
  outline-variant: '#c2c6d6'
  surface-tint: '#005ac2'
  primary: '#0058be'
  on-primary: '#ffffff'
  primary-container: '#2170e4'
  on-primary-container: '#fefcff'
  inverse-primary: '#adc6ff'
  secondary: '#5f5e5e'
  on-secondary: '#ffffff'
  secondary-container: '#e2dfde'
  on-secondary-container: '#636262'
  tertiary: '#924700'
  on-tertiary: '#ffffff'
  tertiary-container: '#b75b00'
  on-tertiary-container: '#fffbff'
  error: '#ba1a1a'
  on-error: '#ffffff'
  error-container: '#ffdad6'
  on-error-container: '#93000a'
  primary-fixed: '#d8e2ff'
  primary-fixed-dim: '#adc6ff'
  on-primary-fixed: '#001a42'
  on-primary-fixed-variant: '#004395'
  secondary-fixed: '#e5e2e1'
  secondary-fixed-dim: '#c8c6c5'
  on-secondary-fixed: '#1c1b1b'
  on-secondary-fixed-variant: '#474746'
  tertiary-fixed: '#ffdcc6'
  tertiary-fixed-dim: '#ffb786'
  on-tertiary-fixed: '#311400'
  on-tertiary-fixed-variant: '#723600'
  background: '#f8f9fa'
  on-background: '#191c1d'
  surface-variant: '#e1e3e4'
typography:
  display-lg:
    fontFamily: Inter
    fontSize: 48px
    fontWeight: '700'
    lineHeight: 56px
    letterSpacing: -0.02em
  display-lg-mobile:
    fontFamily: Inter
    fontSize: 32px
    fontWeight: '700'
    lineHeight: 40px
    letterSpacing: -0.01em
  headline-md:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
  article-body:
    fontFamily: Source Serif 4
    fontSize: 20px
    fontWeight: '400'
    lineHeight: 32px
  article-body-mobile:
    fontFamily: Source Serif 4
    fontSize: 18px
    fontWeight: '400'
    lineHeight: 28px
  interface-md:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: '500'
    lineHeight: 24px
  label-sm:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '600'
    lineHeight: 20px
    letterSpacing: 0.01em
  caption:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '400'
    lineHeight: 16px
rounded:
  sm: 0.25rem
  DEFAULT: 0.5rem
  md: 0.75rem
  lg: 1rem
  xl: 1.5rem
  full: 9999px
spacing:
  base: 8px
  container-max: 1200px
  article-max: 720px
  gutter: 24px
  margin-mobile: 16px
  margin-desktop: 40px
---

## Brand & Style

The design system is anchored in the principles of modern minimalism, prioritizing content clarity above all else. It is designed for a professional blogging environment where the reader's focus is the primary asset. The aesthetic is clean, sophisticated, and intentional, utilizing generous whitespace to create a sense of calm and intellectual rigor.

The style leverages a high-contrast editorial approach: sharp typography and a restricted color palette convey authority, while soft shadows and subtle transitions ensure the interface feels approachable and "human." The transition between the reader-facing frontend and the administrative backend is handled through tonal shifts—moving from an open, expansive white canvas for reading to a structured, focused gray environment for creation.

## Colors

The palette is functional and hierarchical.
- **Primary (Modern Blue):** Reserved for interactive elements, signifiers of action, and progress indicators.
- **Secondary (Charcoal):** Used for primary headings and body text on the frontend to ensure maximum legibility and a premium feel.
- **Neutral (Soft Gray):** Functions as the substrate for the administrative interface, providing a distinct visual mental model for "Management Mode" versus "Reading Mode."
- **Feedback Colors:** Use standard semantic greens (success) and reds (error), but desaturated to match the sophisticated tone of the system.

## Typography

The typography system uses a dual-font strategy to balance utility and immersion.
- **Inter (Sans-Serif):** Used for all functional interface elements, navigation, metadata, and the administrative dashboard. It provides a technical, efficient feel.
- **Source Serif 4 (Serif):** Used exclusively for long-form article content. The increased line height (1.6x) and generous font size are optimized for prolonged reading sessions and "deep work" consumption.

Headings should use tight letter spacing for a modern, "locked-in" look, while body text remains neutral to facilitate flow.

## Layout & Spacing

The layout utilizes a 12-column grid for the dashboard and discovery pages, but shifts to a single-column, centered "focused" layout for article reading.
- **Reading View:** The content width is capped at 720px to maintain optimal characters-per-line (CPL), preventing eye fatigue.
- **Dashboard View:** A fluid grid with a sidebar (fixed at 280px) for administrative tools.
- **Rhythm:** All spacing (margins, padding, gaps) follows an 8px baseline grid. Large sections should be separated by at least 80px (10 units) to maintain the minimalist breathability.

## Elevation & Depth

Elevation in the design system is subtle, avoiding heavy shadows in favor of "ambient depth."
- **Level 0 (Flat):** Primary canvas (Frontend background).
- **Level 1 (Subtle):** Cards and container elements. Uses a 1px border (#E2E8F0) and a very soft shadow: `0px 1px 3px rgba(0,0,0,0.05)`.
- **Level 2 (Interactive):** Hover states for cards. The shadow deepens slightly and the element lifts 2px: `0px 10px 15px -3px rgba(0,0,0,0.1)`.
- **Level 3 (Overlay):** Modals and dropdowns. These use a more pronounced shadow and a backdrop blur (8px) on the layers beneath to maintain focus.

## Shapes

The design system uses a consistent 8px (0.5rem) corner radius for most UI components (buttons, input fields, cards). This "Rounded" setting provides a professional yet contemporary feel that softens the high-contrast typography. Smaller elements like tags or chips may use a fully rounded (pill) shape to distinguish them from actionable buttons.

## Components

- **Buttons:** Primary buttons use the Modern Blue background with white text. Secondary buttons use a subtle gray border with charcoal text. Hover states should involve a slight darkening of the background color rather than a glow.
- **Input Fields:** Use a white background even on the admin dashboard to "pop" against the soft gray. Focus states must use a 2px Modern Blue outline.
- **Cards:** For blog posts, cards should be "borderless" but sit on Level 1 elevation. Images within cards should have a top-rounded 8px radius to match the container.
- **Lists:** Admin lists should use generous vertical padding (16px) and subtle dividers to ensure dense data remains scannable.
- **Chips/Tags:** Small, capitalized text in Inter, using a light blue tint (#EBF3FF) for the background to denote category associations without competing with primary actions.
- **Article Progress Bar:** A thin (4px) Modern Blue bar at the very top of the viewport that fills as the user scrolls, providing a non-intrusive feedback loop for long-form content.
