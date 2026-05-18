# FLUX Design System (Default)

The web app uses a small set of **branding variables** (set by the backend + `apps/web/src/state/branding.ts`) and derives a richer set of **FLUX design tokens** from them in CSS.

## Branding inputs (source of truth)

Set on `:root`:

- `--brand-primary`, `--brand-secondary`
- `--app-bg`, `--app-surface`, `--app-border`, `--app-text`, `--app-muted`

These are what brand presets (Work/Play + theme) and org branding PATCHes ultimately control.

## FLUX design tokens (derived)

Defined in `apps/web/src/styles.css` and intended for all UI:

- Palette/semantic: `--flux-color-accent`, `--flux-color-bg`, `--flux-color-surface-1`, `--flux-color-text`, `--flux-color-muted`, `--flux-color-border`
- Surfaces: `--flux-surface-0/1/2`, `--flux-surface-overlay`
- Typography: `--flux-font-*`
- Radii: `--flux-radius-*`
- Shadows: `--flux-shadow-*`
- Motion: `--flux-duration-*`, `--flux-ease-*`
- Mode overlays: `--flux-overlay-work/play`, `--flux-overlay-active` (selected via `data-ui-mode`)

Because the tokens are CSS-variable references to the branding inputs, **brand preset changes automatically update the whole token set** without additional JS wiring.

## Prefer using token utilities in new UI

These utilities live in `apps/web/src/styles.css` and exist to avoid hard-coded Tailwind colors in primary shells/screens:

- `flux-btn-primary`
- `flux-link`
- `flux-input` + `flux-focus-within`
- `flux-chip-active`
- `flux-status-success|warning|danger`
- `flux-text-success|warning|danger`
- `flux-dot-online|offline`

