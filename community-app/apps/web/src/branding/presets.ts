export type UIMode = "work" | "play";

export type ThemePreset = {
  id: string;
  label: string;
  description: string;
  colorScheme: "dark" | "light";
  vars: {
    brandPrimary: string;
    brandSecondary: string;
    onPrimary: string;
    appBg: string;
    appSurface: string;
    appBorder: string;
    appText: string;
    appMuted: string;
  };
};

export const THEME_PRESETS: ThemePreset[] = [
  // ─── Default ───────────────────────────────────────────────────────────────
  {
    id: "teams-dark",
    label: "Teams Dark",
    description: "Microsoft Teams–inspired dark theme with signature purple accent.",
    colorScheme: "dark",
    vars: {
      brandPrimary:   "#6264a7",
      brandSecondary: "#5b5fc7",
      onPrimary:      "#ffffff",
      appBg:          "#1f1f1f",
      appSurface:     "#292929",
      appBorder:      "#3d3c42",
      appText:        "#f3f2f1",
      appMuted:       "#a6a7a9",
    },
  },

  // ─── Dark ──────────────────────────────────────────────────────────────────
  {
    id: "ember",
    label: "Ember",
    description: "Amber and crimson glowing through warm charcoal stone.",
    colorScheme: "dark",
    vars: {
      brandPrimary:   "#f59e0b",
      brandSecondary: "#ef4444",
      onPrimary:      "#111827",
      appBg:          "#1c1917",
      appSurface:     "#292422",
      appBorder:      "#44403c",
      appText:        "#f5f5f4",
      appMuted:       "#a8a29e",
    },
  },
  {
    id: "cobalt",
    label: "Cobalt",
    description: "Electric blue and cyan in deep slate — crisp and precise.",
    colorScheme: "dark",
    vars: {
      brandPrimary:   "#3b82f6",
      brandSecondary: "#22d3ee",
      onPrimary:      "#ffffff",
      appBg:          "#0f172a",
      appSurface:     "#1e293b",
      appBorder:      "#334155",
      appText:        "#e2e8f0",
      appMuted:       "#94a3b8",
    },
  },
  {
    id: "emerald",
    label: "Emerald",
    description: "Teal green and mint rising from deep forest shadows.",
    colorScheme: "dark",
    vars: {
      brandPrimary:   "#10b981",
      brandSecondary: "#34d399",
      onPrimary:      "#0f172a",
      appBg:          "#0a1612",
      appSurface:     "#141e19",
      appBorder:      "#1e3728",
      appText:        "#ecfdf5",
      appMuted:       "#7fb89e",
    },
  },
  {
    id: "plum",
    label: "Plum",
    description: "Violet and rose across a deep galactic canvas.",
    colorScheme: "dark",
    vars: {
      brandPrimary:   "#a855f7",
      brandSecondary: "#ec4899",
      onPrimary:      "#ffffff",
      appBg:          "#12101c",
      appSurface:     "#1c1930",
      appBorder:      "#362e59",
      appText:        "#f5f3ff",
      appMuted:       "#c4b5fd",
    },
  },
  {
    id: "sunset",
    label: "Sunset",
    description: "Warm orange and rose igniting a dark, amber sky.",
    colorScheme: "dark",
    vars: {
      brandPrimary:   "#f97316",
      brandSecondary: "#f43f5e",
      onPrimary:      "#111827",
      appBg:          "#1a0f09",
      appSurface:     "#261812",
      appBorder:      "#4a2c18",
      appText:        "#fef3c7",
      appMuted:       "#c49a74",
    },
  },

  // ─── Light ─────────────────────────────────────────────────────────────────
  {
    id: "kinetic",
    label: "Kinetic",
    description: "Warm amber and crimson on cream — kinetic energy in daylight.",
    colorScheme: "light",
    vars: {
      brandPrimary:   "#b45309",
      brandSecondary: "#dc2626",
      onPrimary:      "#ffffff",
      appBg:          "#fffdf6",
      appSurface:     "#ffffff",
      appBorder:      "#eedcb0",
      appText:        "#1c1611",
      appMuted:       "#967c52",
    },
  },
  {
    id: "ocean",
    label: "Ocean",
    description: "Sky blue and cyan on a crisp blue-white horizon.",
    colorScheme: "light",
    vars: {
      brandPrimary:   "#0284c7",
      brandSecondary: "#0891b2",
      onPrimary:      "#ffffff",
      appBg:          "#f0f9ff",
      appSurface:     "#ffffff",
      appBorder:      "#bae6fd",
      appText:        "#0c1a2e",
      appMuted:       "#5d87a0",
    },
  },
  {
    id: "forest",
    label: "Forest",
    description: "Deep green and teal on soft sage — the stillness of old growth.",
    colorScheme: "light",
    vars: {
      brandPrimary:   "#15803d",
      brandSecondary: "#0d9488",
      onPrimary:      "#ffffff",
      appBg:          "#f0fdf4",
      appSurface:     "#ffffff",
      appBorder:      "#bbf7d0",
      appText:        "#0d1f12",
      appMuted:       "#4a7a5a",
    },
  },
  {
    id: "aurora",
    label: "Aurora",
    description: "Teal and emerald rippling across clean, airy surfaces.",
    colorScheme: "light",
    vars: {
      brandPrimary:   "#0d9488",
      brandSecondary: "#059669",
      onPrimary:      "#ffffff",
      appBg:          "#f0fdfa",
      appSurface:     "#ffffff",
      appBorder:      "#99f6e4",
      appText:        "#0d1f1d",
      appMuted:       "#3d7d72",
    },
  },
  {
    id: "lavender",
    label: "Lavender",
    description: "Rich violet and amethyst on a soft, dreamy lavender base.",
    colorScheme: "light",
    vars: {
      brandPrimary:   "#7c3aed",
      brandSecondary: "#a855f7",
      onPrimary:      "#ffffff",
      appBg:          "#faf5ff",
      appSurface:     "#ffffff",
      appBorder:      "#e9d5ff",
      appText:        "#1e0d2e",
      appMuted:       "#7a5a9e",
    },
  },
  {
    id: "slate",
    label: "Slate",
    description: "Cool, neutral slate — minimal, professional, no distractions.",
    colorScheme: "light",
    vars: {
      brandPrimary:   "#334155",
      brandSecondary: "#475569",
      onPrimary:      "#ffffff",
      appBg:          "#f8fafc",
      appSurface:     "#ffffff",
      appBorder:      "#e2e8f0",
      appText:        "#0f172a",
      appMuted:       "#64748b",
    },
  },
];

export const DEFAULT_THEME_ID = "teams-dark";

export function getThemePreset(themeId?: string): ThemePreset {
  return (
    THEME_PRESETS.find((t) => t.id === (themeId ?? DEFAULT_THEME_ID)) ??
    THEME_PRESETS[0]
  );
}

export const COLOR_PALETTES = {
  primary: [
    // purples / blues
    "#6264a7",
    "#5b5fc7",
    "#7c3aed",
    "#a855f7",
    "#3b82f6",
    "#0284c7",
    "#0891b2",
    "#22d3ee",
    // greens / teals
    "#10b981",
    "#34d399",
    "#0d9488",
    "#059669",
    "#15803d",
    // warm
    "#f59e0b",
    "#b45309",
    "#f97316",
    "#ef4444",
    "#dc2626",
    "#f43f5e",
    "#ec4899",
    // neutrals
    "#334155",
    "#475569",
    "#64748b",
  ],
  background: [
    // dark
    "#1f1f1f",
    "#1c1917",
    "#0f172a",
    "#0a1612",
    "#12101c",
    "#1a0f09",
    "#111827",
    // light
    "#fffdf6",
    "#f0f9ff",
    "#f0fdf4",
    "#f0fdfa",
    "#faf5ff",
    "#f8fafc",
    "#f1f5f9",
  ],
  surface: [
    // dark
    "#292929",
    "#292422",
    "#1e293b",
    "#141e19",
    "#1c1930",
    "#261812",
    "#1a1f26",
    // light
    "#ffffff",
    "#f8fafc",
    "#f1f5f9",
  ],
  border: [
    // dark
    "#3d3c42",
    "#44403c",
    "#334155",
    "#1e3728",
    "#362e59",
    "#4a2c18",
    "#2d3748",
    "#263044",
    // light
    "#eedcb0",
    "#bae6fd",
    "#bbf7d0",
    "#99f6e4",
    "#e9d5ff",
    "#e2e8f0",
    "#cbd5e1",
  ],
  text: [
    // light on dark
    "#f3f2f1",
    "#f5f5f4",
    "#e2e8f0",
    "#ecfdf5",
    "#f5f3ff",
    "#fef3c7",
    // dark on light
    "#1c1611",
    "#0c1a2e",
    "#0d1f12",
    "#0d1f1d",
    "#1e0d2e",
    "#0f172a",
  ],
  muted: [
    // dark
    "#a6a7a9",
    "#a8a29e",
    "#94a3b8",
    "#7fb89e",
    "#c4b5fd",
    "#c49a74",
    "#8090a5",
    // light
    "#967c52",
    "#5d87a0",
    "#4a7a5a",
    "#3d7d72",
    "#7a5a9e",
    "#64748b",
    "#475569",
  ],
};
