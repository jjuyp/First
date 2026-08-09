# Codex UI Implementation Contract

Before implementing UI, read:
1. `design/STARROOM_DESIGN_DNA.json`
2. `docs/13_UI_UX_SPEC.md`
3. `docs/14_UI_MOTION_PERFORMANCE.md`
4. `AGENTS.md`

## Requirements

- Implement Design DNA as typed CSS variables/tokens.
- Dark, Gray, Light themes share semantic token names.
- Do not copy third-party app UI assets or proprietary Adobe visual assets.
- Implement Starroom's specified layout independently.
- Start with static shell fidelity before motion.
- Implement functional interactions before decorative glass.
- Make UI effects feature-toggleable by performance mode.
- Use one underlying editor state for Simple and Pro modes.
- Never put CSS filter effects over the rendered image preview.
- Do not use Canvas/WebGL for decorative UI; GPU is reserved for the image renderer.
- Persist theme, panel collapse states, panel widths and last Simple/Pro mode.
- First-ever mode is Simple.
