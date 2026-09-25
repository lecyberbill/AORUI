---
name: AetherOS
colors:
  surface: '#0f131d'
  surface-dim: '#0f131d'
  surface-bright: '#353944'
  surface-container-lowest: '#0a0e18'
  surface-container-low: '#171b26'
  surface-container: '#1c1f2a'
  surface-container-high: '#262a35'
  surface-container-highest: '#313540'
  on-surface: '#dfe2f1'
  on-surface-variant: '#c7c4d7'
  inverse-surface: '#dfe2f1'
  inverse-on-surface: '#2c303b'
  outline: '#908fa0'
  outline-variant: '#464554'
  surface-tint: '#c0c1ff'
  primary: '#c0c1ff'
  on-primary: '#1000a9'
  primary-container: '#8083ff'
  on-primary-container: '#0d0096'
  inverse-primary: '#494bd6'
  secondary: '#7bd0ff'
  on-secondary: '#00354a'
  secondary-container: '#00a6e0'
  on-secondary-container: '#00374d'
  tertiary: '#4edea3'
  on-tertiary: '#003824'
  tertiary-container: '#00885d'
  on-tertiary-container: '#000703'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#e1e0ff'
  primary-fixed-dim: '#c0c1ff'
  on-primary-fixed: '#07006c'
  on-primary-fixed-variant: '#2f2ebe'
  secondary-fixed: '#c4e7ff'
  secondary-fixed-dim: '#7bd0ff'
  on-secondary-fixed: '#001e2c'
  on-secondary-fixed-variant: '#004c69'
  tertiary-fixed: '#6ffbbe'
  tertiary-fixed-dim: '#4edea3'
  on-tertiary-fixed: '#002113'
  on-tertiary-fixed-variant: '#005236'
  background: '#0f131d'
  on-background: '#dfe2f1'
  surface-variant: '#313540'
typography:
  display-lg:
    fontFamily: Plus Jakarta Sans
    fontSize: 48px
    fontWeight: '700'
    lineHeight: 56px
    letterSpacing: -0.03em
  display-lg-mobile:
    fontFamily: Plus Jakarta Sans
    fontSize: 32px
    fontWeight: '700'
    lineHeight: 40px
    letterSpacing: -0.02em
  headline-lg:
    fontFamily: Plus Jakarta Sans
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
    letterSpacing: -0.02em
  headline-md:
    fontFamily: Plus Jakarta Sans
    fontSize: 18px
    fontWeight: '600'
    lineHeight: 26px
    letterSpacing: -0.015em
  headline-sm:
    fontFamily: Plus Jakarta Sans
    fontSize: 15px
    fontWeight: '600'
    lineHeight: 22px
    letterSpacing: -0.01em
  body-lg:
    fontFamily: Inter
    fontSize: 15px
    fontWeight: '400'
    lineHeight: 24px
  body-md:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: '400'
    lineHeight: 20px
  body-sm:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '400'
    lineHeight: 18px
  mono-lg:
    fontFamily: JetBrains Mono
    fontSize: 13px
    fontWeight: '500'
    lineHeight: 18px
    letterSpacing: -0.01em
  mono-sm:
    fontFamily: JetBrains Mono
    fontSize: 11px
    fontWeight: '500'
    lineHeight: 16px
    letterSpacing: 0em
  label-md:
    fontFamily: Inter
    fontSize: 11px
    fontWeight: '600'
    lineHeight: 16px
    letterSpacing: 0.04em
  label-sm:
    fontFamily: Inter
    fontSize: 10px
    fontWeight: '600'
    lineHeight: 14px
    letterSpacing: 0.06em
rounded:
  sm: 0.25rem
  DEFAULT: 0.5rem
  md: 0.75rem
  lg: 1rem
  xl: 1.5rem
  full: 9999px
spacing:
  gutter: 1rem
  gutter-sm: 0.5rem
  margin: 1.5rem
  margin-mobile: 0.75rem
  space-xs: 0.25rem
  space-sm: 0.5rem
  space-md: 0.75rem
  space-lg: 1.25rem
  space-xl: 2rem
---

## Brand & Style

This design system defines a next-generation cloud operating system interface designed to unify native desktop power with cloud ubiquity. The emotional tone evokes calm mastery, weightless fluid precision, and high-performance capability. It rejects messy, bloated desktop metaphors in favor of an ethereal, hyper-focused spatial workspace.

The design movement combines **Deep Atmospheric Glassmorphism** with **Technical Precision**. It avoids excessive, illegible frosted blurs, using instead strictly measured translucent surface planes, razor-thin luminous hairline borders (1px), subtle ambient light falloff, and hyper-legible typographic rhythms. Every window, widget, and panel exists as an optical glass stratum floating above a cosmic twilight horizon.

## Colors

The palette establishes an infinite deep-space backdrop with high-voltage luminous accents, engineered to prevent eye fatigue during continuous operational sessions.

### Surface Tokens & Glass Roles
- **Canvas Base (`#0B0F19`)**: The primary root desktop canvas.
- **Surface Elevation 1 (`#111827` at 75% opacity with 24px backdrop-blur)**: Floating windows, secondary sidebars, and inactive panels.
- **Surface Elevation 2 (`#1E293B` at 65% opacity with 32px backdrop-blur)**: Floating dock, top menu bar, and context flyouts.
- **Surface Elevation 3 (`#334155` at 80% opacity with 40px backdrop-blur)**: Modal dialogues, command palettes, and active window headers.

### Accent & Functional Colors
- **Primary (`#6366F1` - Electric Indigo)**: Primary interaction points, active window borders, system accents, and focal highlights.
- **Secondary (`#38BDF8` - Cyan Ray)**: Network throughput indicators, active tabs, telemetry highlights, and selected file items.
- **Tertiary (`#10B981` - Emerald Sync)**: Cloud sync status, operational system health, and positive confirmations.
- **Critical / Accent (`#F43F5E` - Rose Core)**: Error signals, resource thresholds, and destructive alerts.

### Boundary & Structural Tones
- **Subtle Glass Border**: `rgba(255, 255, 255, 0.08)` on inactive elements, shifting to `rgba(56, 189, 248, 0.25)` or `rgba(99, 102, 241, 0.35)` on active focus.
- **Inner Light Rim**: Top-edge inset highlight `inset 0 1px 0 0 rgba(255, 255, 255, 0.12)` delivering physical glass edge refraction.

## Typography

The typographic hierarchy establishes clear informational stratification across dense user interfaces:
- **Headlines (`Plus Jakarta Sans`)**: Delivers geometric warmth and human balance to structural frame titles, window headers, and modal prompts.
- **Body & Controls (`Inter`)**: Guarantees legibility at micro scales (11px–13px) across multi-column data views, file hierarchies, and inspector panes.
- **Telemetry & Metadata (`JetBrains Mono`)**: Dedicated to system metrics (CPU load, network throughput, memory graphs, clock time, storage bytes, and code buffers).

Always enforce tabular figures (`font-variant-numeric: tabular-nums`) on telemetry and data monitors to prevent spatial jitter during real-time data streaming.

## Layout & Spacing

This design system implements a **Contextual Spatial Desktop Layout** anchored by two persistent architectural zones:
1. **Top Global Bar**: Fixed height of 36px, zero-margin, edge-to-edge spanning horizontally. Houses system controls, workspace indicator, clock, and telemetry strip.
2. **Floating Action Dock**: Dynamic width, auto-centering horizontally at the bottom edge with a persistent bottom margin of `1rem` (16px).
3. **Workspace Canvas**: Fluid viewport calculation (`height: calc(100vh - 36px)`), serving as the freeform floating canvas for window managers or auto-tiling arrangements.

### Window Tiling & Adaptive Viewports
- **Desktop (>= 1200px)**: Free-floating overlapping windows with arbitrary resizing, snap-assist grids (halves, quadrants, and thirds), and floating inspector drawers.
- **Tablet (768px - 1199px)**: Automatic side-by-side or stacked split panes. Top global bar remains; the dock converts to an autohiding slide-over gesture strip.
- **Mobile (< 768px)**: Converts from a freeform window canvas into a tabbed single-app immersion mode. The top menu bar simplifies to system time, network, and battery icons; the bottom dock shifts into an app-switcher swipe pill.

## Elevation & Depth

Visual depth is achieved through an optical glass hierarchy combining variable backdrop blurs, surface opacity layers, and soft, tinted ambient shadows.

### Atmospheric Glass Levels
- **Canvas Base Layer**: `#0B0F19` backdrop with subtle radial gradients of indigo/cyan (`rgba(99, 102, 241, 0.04)` to `rgba(56, 189, 248, 0.02)`) anchored at screen peripheries.
- **Elevation Level 1 (Windows & Workspaces)**:
  - Background: `rgba(17, 24, 39, 0.72)`
  - Filter: `backdrop-filter: blur(20px) saturate(160%)`
  - Border: `1px solid rgba(255, 255, 255, 0.08)`
  - Shadow: `0 8px 32px 0 rgba(0, 0, 0, 0.36), 0 2px 8px 0 rgba(0, 0, 0, 0.24)`
- **Elevation Level 2 (Focused Window / System Trays / Dock)**:
  - Background: `rgba(30, 41, 59, 0.78)`
  - Filter: `backdrop-filter: blur(28px) saturate(180%)`
  - Border: `1px solid rgba(99, 102, 241, 0.30)`
  - Shadow: `0 16px 48px -4px rgba(0, 0, 0, 0.5), 0 0 24px 0 rgba(99, 102, 241, 0.15)`
- **Elevation Level 3 (Context Menus, Flyouts & Modals)**:
  - Background: `rgba(15, 23, 42, 0.88)`
  - Filter: `backdrop-filter: blur(36px) saturate(200%)`
  - Border: `1px solid rgba(255, 255, 255, 0.15)`
  - Shadow: `0 24px 64px 0 rgba(0, 0, 0, 0.65), inset 0 1px 0 0 rgba(255, 255, 255, 0.2)`

## Shapes

The interface embraces controlled, organic curvature:
- **Base Components (0.5rem / 8px)**: Input fields, list row selection states, buttons, context menus, and small widget cards.
- **Containers & Windows (1rem / 16px)**: Floating application windows, notification banners, large settings sheets, and floating modal boxes.
- **Pill Shells (9999px)**: Floating bottom dock, top menu status badges, tag chips, and slider knobs.

## Components

### Window Chrome & Frame
- **Header**: 38px height, translucent background, draggable region. Houses traffic light control buttons (12px circular controls: Close `#F43F5E`, Minimize `#F59E0B`, Maximize `#10B981`) on the left, centered title typography (`headline-sm`), and active app workspace tools on the right.
- **Active State**: The focused window transitions its border from `rgba(255, 255, 255, 0.08)` to `rgba(99, 102, 241, 0.35)` with an interior edge highlight `inset 0 1px 0 0 rgba(255, 255, 255, 0.15)`.

### Top Global Menu Bar
- **Dimensions & Style**: Height 36px, `rgba(11, 15, 25, 0.8)` with 16px blur, bottom hairline border `1px solid rgba(255, 255, 255, 0.06)`.
- **System Telemetry Module**: Displays compact mono readings (`mono-sm`) for CPU % and RAM % with inline sparklines or micro progress loops, ping counter, and a pulsing tertiary `#10B981` dot for cloud sync status.
- **Quick Settings Trigger**: Combined pill grouping network, sound, and battery indicators with immediate hover reaction.

### Floating Dock
- **Layout**: Self-centering horizontal shelf at the bottom, padded with `space-xs` (4px), inner glass layer `rgba(30, 41, 59, 0.65)`.
- **App Icons**: 44x44px rounded squares (10px radius) with interactive smooth magnification on proximity hover. Running applications display a tiny 3px glowing cyan pill beneath the icon.

### Buttons & Interactive Controls
- **Primary Action**: Solid `#6366F1` background, white text, subtle hover lift, accompanied by an ambient glow `0 0 16px rgba(99, 102, 241, 0.4)`.
- **Ghost / Glass Action**: Translucent fill `rgba(255, 255, 255, 0.05)`, border `1px solid rgba(255, 255, 255, 0.1)`, shifting to `rgba(255, 255, 255, 0.12)` fill on pointer hover.
- **Input Fields**: Height 34px, background `rgba(0, 0, 0, 0.25)`, border `1px solid rgba(255, 255, 255, 0.1)`. Focus state produces a 1px border `#38BDF8` with a soft outer ring `0 0 0 2px rgba(56, 189, 248, 0.2)`.

### File Manager & Workspace Lists
- **Grid / Tree View**: Alternating item hover states using `rgba(255, 255, 255, 0.04)`. Selected row applies `rgba(99, 102, 241, 0.15)` with a left accent stripe in `#38BDF8`.
- **File Metadata**: Micro text in `mono-sm` for byte sizes, dates, and file permissions.

### Notification Toast
- Fixed bottom-right anchor, slide-up physics. Glass elevation level 3, distinct app icon indicator, actionable buttons inlined horizontally with crisp hairline dividers.