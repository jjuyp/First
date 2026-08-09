# Starroom UI Specification — Studio DNA v1

## 1. Locked Design Direction
- Overall: Modern Studio
- Ratio: 75% professional modern workstation + 25% restrained future glass
- Themes: Dark / Gray / Light
- Default theme: Dark
- Left Library: collapsible
- Bottom Filmstrip: collapsible
- Right Tools: icon category rail + accordion inspector
- Mask UX: right-side mask management + compact floating canvas toolbar
- Motion: restrained 100–280 ms, no bounce
- Glass: visually present but computationally budgeted
- Corner radius: 6 / 8 / 10 px
- UI font: Segoe UI Variable
- Numeric font: JetBrains Mono / Cascadia Mono fallback
- Icons: line default, filled/hybrid selected
- Slider: accent-fill left track + hover/drag numeric bubble
- Simple/Pro: first launch Simple, later restore previous mode
- Canvas: neutral background + only a very subtle vignette
- Photo is always the dominant visual element

## 2. Brand Accent
```css
--starroom-brand-gradient: linear-gradient(
  to right top,
  #f891ef,
  #d3a8ff,
  #adbaff,
  #92c8ff,
  #8ad2ff,
  #8ac8f0,
  #89bde2,
  #87b3d3,
  #7e95b0,
  #717a8c,
  #5f6069,
  #494949
);
```
Allowed full-gradient uses: logo/brand, AI features, Look Engine, Style Mixer, Reference Match, onboarding accents, selected creative-mode edge and a restrained primary Export action.
Do not use the full gradient for every slider, heading, persistent panel, histogram or canvas.
Functional accent subset: Lavender `#D3A8FF`, Periwinkle `#ADBaff`, Ice Blue `#8AD2FF`.

## 3. Themes
### Dark
canvas shell `#111214`; surfaces `#17181B/#202126/#292B30/#34363B`; text `#F4F4F5/#B8BBC2/#80838B`.
### Gray
canvas shell `#292B2F`; surfaces `#34363A/#3E4045/#494B50/#55575C`; text `#F4F4F5/#D0D2D6/#A0A3AA`.
### Light
canvas shell `#D8D9DC`; surfaces `#ECEDEF/#F5F5F6/#FFFFFF/#E7E8EB`; text `#202126/#555960/#777B83`. Light canvas is never pure white.

## 4. Proof Background
Independent from theme: Black, Dark Gray, 50% Gray, White.

## 5. Main Shell
```text
┌─────────────────────────────────────────────────────────────────────────┐
│ Starroom | Library | Edit | Compare         Simple/Pro   Undo   Export │
├──────────────┬────────────────────────────────────────┬─────────────────┤
│ LEFT LIBRARY │                                        │ TOOL CATEGORY   │
│ collapsible  │              PHOTO CANVAS              │ + INSPECTOR     │
│ folders      │                                        │ Histogram       │
│ albums       │                                        │ Light/Color     │
│ recent       │                                        │ Curve/Detail    │
│              │                                        │ Masks/Optics    │
├──────────────┴────────────────────────────────────────┴─────────────────┤
│ FILMSTRIP — collapsible thumbnails                                    │
└─────────────────────────────────────────────────────────────────────────┘
```
At >=1440px: top bar 44px; left library 220–260px resizable; right inspector 320–380px resizable; category rail 44–48px; filmstrip 108–144px; center canvas flexible.
Compact-width collapse priority: left Library, then inspector reduction, then filmstrip. Preserve central canvas usability first.

## 6. Top Bar
Left: logo, Library, Edit, Compare. Center/right: Simple/Pro segmented toggle, Before/After, Undo, Redo, degraded-performance indicator only when needed, Export. Native menus may remain accessible without cluttering the visible shell.

## 7. Right Tool Architecture
Category rail: Light, Color, Curve, Detail, Masks, Optics, Geometry. Selected category uses filled/hybrid icon and subtle lavender/blue accent. Inspector uses accordion groups rather than one extremely long panel.

## 8. Slider
Rest: 2px track, neutral unfilled region, functional-accent filled region, 10–12px knob.
Hover: knob slightly enlarges and floating numeric tooltip appears above knob using tabular numerals.
Drag: pointer capture, tooltip persists; Escape restores pre-drag value; double-click resets; Shift drag fine adjustment; optional Alt extra-fine after testing. Never animate value lag.

## 9. Mask UX
Selecting Masks opens right mask stack and a transient glass canvas toolbar:
```text
╭─────────────────────────────────────────╮
│ Brush | Radial | Linear | Color | Luma │
│ Subject | Person | Sky | Background    │
╰─────────────────────────────────────────╯
```
After selection, toolbar collapses to an edit chip and mask appears in right tree. Mask tree supports Person submasks, nested Subtract/Intersect operations, Add/Subtract/Intersect actions and overlays Red / White-on-Black / Black-on-White / Grayscale. `O` toggles overlay. Persistent mask inspector stays mostly opaque.

## 10. Simple Mode
Sections: Light, Color, Skin, Mood, Detail, Background. Mood axes: Clean↔Moody, Soft↔Crisp, Warm↔Cool, Natural↔Cinematic, Airy↔Rich, Modern↔Vintage. These map to the same typed Pro parameters.

## 11. Pro Mode
Basic, Curve, Color Mixer, Color Grading, Calibration, Detail, Optics, Geometry, Masks. Switching mode never changes image output by itself.

## 12. Look Engine UI
Brand-expressive browser with categories, thumbnail cards, Amount slider, optional Style Mixer and applied-parameter summary. Selected Look edge may use brand gradient. No animated full-panel gradient.

## 13. Reference Match UI
Two-image Target/Reference workspace. Controls: Tone, Color, Full Look, Match Strength, per-component toggles and confidence indicators. After matching, show explainable changes such as Exposure +0.22 EV, Highlights -14, Blue Hue +7, Shadow grade cooler.

## 14. AI Denoise UI
Keep inside Detail panel, not a giant AI dashboard: Strength slider, Preview Region, Apply, current device. Optional advanced detail retention and diagnostics. Preview visible crop first.

## 15. Library + Filmstrip
Library collapsible/resizable with folders/recent/albums/collections later. Filmstrip horizontal/collapsible with rating/flag overlays, 2px active accent border and clear multiselect state.

## 16. Compare
Before/After, side-by-side, split, reference and survey. Chrome may reduce opacity while pointer is over image, but keyboard access remains.

## 17. Glass Policy
Real backdrop blur allowed only for transient surfaces: mask floating toolbar, context menu, tooltip, Look picker, Reference Match floating actions, AI suggestions and short-lived modals. Avoid real blur on full Library, inspector, filmstrip, image canvas and persistent top bar. Faux-glass uses alpha surface, highlight border, compact shadow and static subtle gradient.

## 18. Accessibility
WCAG AA; keyboard navigation; visible focus; reduced-motion support; color is never the only signal; mask overlay color may be customizable later; pointer targets practical minimum 28px, preferred 32–36px.

## 19. UI Quality Modes
Auto default. High: transient blur, full subtle shadows, 15Hz histogram. Balanced: reduced blur/shadow, 10Hz histogram. Performance: blur off, reduced animation/shadow, 5–8Hz histogram while dragging, pause AI background suggestions. Final export quality never changes because of UI mode.

## 20. Canvas
Flat neutral background, extremely subtle outer-canvas vignette only, no decorative gradient, no noise texture, checkerboard only for transparency. Never put CSS effects over image pixels.

## 21. Performance Rule
Any decorative UI effect must be removable without changing image appearance, editing semantics, project state or export.
