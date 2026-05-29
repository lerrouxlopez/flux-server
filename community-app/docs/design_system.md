# FLUX Design System

The web app uses a small set of **branding variables** (set by the backend + `apps/web/src/state/branding.ts`) and derives a richer set of **FLUX design tokens** from them in CSS.

## Two orthogonal layers

| Layer | What it controls | Who owns it |
|-------|-----------------|-------------|
| **Theme** | All visual styling — colors, color scheme, surfaces | User preference |
| **Mode** (Work / Play) | Feature toggles — media defaults, density, notifications, voice drop | User preference |

These are independent. Changing mode does **not** change the theme. Changing theme does **not** change mode behavior.

## Branding inputs (source of truth)

Set on `:root` via `applyBrandingToDom()`:

- `--brand-primary`, `--brand-secondary`
- `--app-bg`, `--app-surface`, `--app-border`, `--app-text`, `--app-muted`

## FLUX design tokens (derived)

Defined in `apps/web/src/styles.css` and intended for all UI:

- Palette/semantic: `--flux-color-accent`, `--flux-color-bg`, `--flux-color-surface-1`, `--flux-color-text`, `--flux-color-muted`, `--flux-color-border`
- Surfaces: `--flux-surface-0/1/2`, `--flux-surface-overlay`
- Typography: `--flux-font-*`
- Radii: `--flux-radius-*`
- Shadows: `--flux-shadow-*`
- Motion: `--flux-duration-*`, `--flux-ease-*`

Because the tokens are CSS-variable references to the branding inputs, **any theme change automatically updates the whole token set** without additional JS wiring.

## Theme system

21 preset themes defined in `apps/web/src/branding/presets.ts`, each with:

```ts
type ThemePreset = {
  id: string;
  label: string;
  description: string;
  colorScheme: "dark" | "light";
  vars: {
    brandPrimary: string;
    brandSecondary: string;
    onPrimary: string;   // text color on primary buttons (#fff or #000)
    appBg: string;
    appSurface: string;
    appBorder: string;
    appText: string;
    appMuted: string;
  };
};
```

### Available themes

| ID | Label | Scheme |
|----|-------|--------|
| `teams-dark` *(default)* | Teams Dark | dark |
| `indigo-focus` | Indigo Focus | dark |
| `teal-clarity` | Teal Clarity | dark |
| `cobalt-ops` | Cobalt Ops | dark |
| `graphite` | Graphite | dark |
| `amber-signal` | Amber Signal | dark |
| `neon-arcade` | Neon Arcade | dark |
| `magenta-pop` | Magenta Pop | dark |
| `cyber-lime` | Cyber Lime | dark |
| `sunset-runner` | Sunset Runner | dark |
| `royal-purple` | Royal Purple | dark |
| `paper-indigo` | Paper Indigo | light |
| `mist-teal` | Mist Teal | light |
| `cloud-blue` | Cloud Blue | light |
| `ledger-gray` | Ledger Gray | light |
| `sunlit-amber` | Sunlit Amber | light |
| `arcade-day` | Arcade Day | light |
| `bubblegum` | Bubblegum | light |
| `lime-soda` | Lime Soda | light |
| `gold-loot` | Gold Loot | light |
| `mint-chill` | Mint Chill | light |

### Theme preference storage

1. **localStorage** (`flux_theme_preference`) — applied immediately on load
2. **Backend** (`users.experience_theme_preference`) — synced via `PATCH /experience/preferences` when authenticated; read back on `GET /experience/context` as `theme_preference`
3. **Org suggestion** (`branding_profiles.ui_theme`) — fallback for users with no preference; org admin sets via the Branding panel

Priority: user preference > org suggestion > `teams-dark` default.

## Mode system (Work / Play)

Mode is a **feature toggle only** — it has no visual effect. It is managed by `ExperienceProvider` and stored in:

- `localStorage` (`flux_experience_mode_preference`)
- Backend (`users.experience_mode_preference`) via `PATCH /experience/preferences`

What mode controls:
- `density`: comfortable (work) / compact (play)
- `motion`: full (work) / reduced (play)
- `notificationProfile`: all (work) / minimal (play)
- `mediaDefaults`: meeting+video (work) / voice-only (play)
- `featureFlags`: work panes, meeting room enabled on work; voice dock emphasized on play

`data-ui-mode="work|play"` is set on `:root` by `ExperienceProvider` for any feature-flag CSS selectors.

## Prefer using token utilities in new UI

These utilities live in `apps/web/src/styles.css`:

- `flux-btn-primary`
- `flux-link`
- `flux-input` + `flux-focus-within`
- `flux-chip-active`
- `flux-status-success|warning|danger`
- `flux-text-success|warning|danger`
- `flux-dot-online|offline`

Avoid hard-coding Tailwind color classes in primary shells/screens — prefer FLUX token utilities so themes apply correctly.
