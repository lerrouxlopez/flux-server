export type UIMode = "work" | "play";

export type ThemePreset = {
  id: string;
  mode: UIMode;
  label: string;
  description: string;
  colorScheme: "dark" | "light";
  vars: {
    brandPrimary: string;
    brandSecondary: string;
    appBg: string;
    appSurface: string;
    appBorder: string;
    appText: string;
    appMuted: string;
  };
};

// Tailwind "Slate" base with a Material-ish accent mix.
export const THEME_PRESETS: ThemePreset[] = [
  // --- Work (calm, readable, low distraction) ---
  {
    id: "work-01",
    mode: "work",
    label: "Indigo Focus",
    description: "Classic slate + indigo primary for clear hierarchy.",
    colorScheme: "dark",
    vars: {
      brandPrimary: "#4f46e5",
      brandSecondary: "#0f172a",
      appBg: "#020617",
      appSurface: "#0b1220",
      appBorder: "#1f2937",
      appText: "#e2e8f0",
      appMuted: "#94a3b8",
    },
  },
  {
    id: "work-02",
    mode: "work",
    label: "Teal Clarity",
    description: "Softer contrast with a teal accent for long sessions.",
    colorScheme: "dark",
    vars: {
      brandPrimary: "#14b8a6",
      brandSecondary: "#0f172a",
      appBg: "#020617",
      appSurface: "#0a1220",
      appBorder: "#1e293b",
      appText: "#e2e8f0",
      appMuted: "#94a3b8",
    },
  },
  {
    id: "work-03",
    mode: "work",
    label: "Cobalt Ops",
    description: "Blue-forward accent, still professional and restrained.",
    colorScheme: "dark",
    vars: {
      brandPrimary: "#2563eb",
      brandSecondary: "#0f172a",
      appBg: "#020617",
      appSurface: "#0b1220",
      appBorder: "#1f2937",
      appText: "#e2e8f0",
      appMuted: "#93a4b8",
    },
  },
  {
    id: "work-04",
    mode: "work",
    label: "Graphite",
    description: "Minimal accent, maximum neutrality.",
    colorScheme: "dark",
    vars: {
      brandPrimary: "#64748b",
      brandSecondary: "#0b1220",
      appBg: "#020617",
      appSurface: "#070b14",
      appBorder: "#1f2937",
      appText: "#e2e8f0",
      appMuted: "#9aa7b8",
    },
  },
  {
    id: "work-05",
    mode: "work",
    label: "Amber Signal",
    description: "Warm highlight for attention without feeling playful.",
    colorScheme: "dark",
    vars: {
      brandPrimary: "#f59e0b",
      brandSecondary: "#0f172a",
      appBg: "#020617",
      appSurface: "#0b1220",
      appBorder: "#243041",
      appText: "#e2e8f0",
      appMuted: "#a1aab8",
    },
  },
  {
    id: "work-06",
    mode: "work",
    label: "Paper Indigo",
    description: "Light slate surfaces with an indigo primary.",
    colorScheme: "light",
    vars: {
      brandPrimary: "#4f46e5",
      brandSecondary: "#ffffff",
      appBg: "#f8fafc",
      appSurface: "#ffffff",
      appBorder: "#e2e8f0",
      appText: "#0f172a",
      appMuted: "#475569",
    },
  },
  {
    id: "work-07",
    mode: "work",
    label: "Mist Teal",
    description: "Low-glare light theme with teal accents.",
    colorScheme: "light",
    vars: {
      brandPrimary: "#14b8a6",
      brandSecondary: "#ffffff",
      appBg: "#f8fafc",
      appSurface: "#ffffff",
      appBorder: "#e2e8f0",
      appText: "#0f172a",
      appMuted: "#475569",
    },
  },
  {
    id: "work-08",
    mode: "work",
    label: "Cloud Blue",
    description: "Bright workspace with a crisp blue primary.",
    colorScheme: "light",
    vars: {
      brandPrimary: "#2563eb",
      brandSecondary: "#ffffff",
      appBg: "#f8fafc",
      appSurface: "#ffffff",
      appBorder: "#e2e8f0",
      appText: "#0f172a",
      appMuted: "#475569",
    },
  },
  {
    id: "work-09",
    mode: "work",
    label: "Ledger Gray",
    description: "Neutral light slate for finance/ops dashboards.",
    colorScheme: "light",
    vars: {
      brandPrimary: "#334155",
      brandSecondary: "#ffffff",
      appBg: "#f8fafc",
      appSurface: "#ffffff",
      appBorder: "#e2e8f0",
      appText: "#0f172a",
      appMuted: "#475569",
    },
  },
  {
    id: "work-10",
    mode: "work",
    label: "Sunlit Amber",
    description: "Warm, calm highlight for alerts and actions.",
    colorScheme: "light",
    vars: {
      brandPrimary: "#f59e0b",
      brandSecondary: "#ffffff",
      appBg: "#f8fafc",
      appSurface: "#ffffff",
      appBorder: "#e2e8f0",
      appText: "#0f172a",
      appMuted: "#475569",
    },
  },

  // --- Play (more energy, higher saturation) ---
  {
    id: "play-01",
    mode: "play",
    label: "Neon Arcade",
    description: "Electric cyan + deep slate, energetic but readable.",
    colorScheme: "dark",
    vars: {
      brandPrimary: "#22d3ee",
      brandSecondary: "#0b1220",
      appBg: "#050817",
      appSurface: "#0b1220",
      appBorder: "#23304a",
      appText: "#e2e8f0",
      appMuted: "#94a3b8",
    },
  },
  {
    id: "play-02",
    mode: "play",
    label: "Magenta Pop",
    description: "Vibrant magenta accent for playful UI moments.",
    colorScheme: "dark",
    vars: {
      brandPrimary: "#d946ef",
      brandSecondary: "#0b1220",
      appBg: "#07051a",
      appSurface: "#120b24",
      appBorder: "#2b1b49",
      appText: "#f1f5f9",
      appMuted: "#a5b4fc",
    },
  },
  {
    id: "play-03",
    mode: "play",
    label: "Cyber Lime",
    description: "Lime accent with a cool slate baseline.",
    colorScheme: "dark",
    vars: {
      brandPrimary: "#a3e635",
      brandSecondary: "#0b1220",
      appBg: "#050a10",
      appSurface: "#0b1220",
      appBorder: "#26404a",
      appText: "#e2e8f0",
      appMuted: "#93c5fd",
    },
  },
  {
    id: "play-04",
    mode: "play",
    label: "Sunset Runner",
    description: "Warm gradient feel via orange/red accents.",
    colorScheme: "dark",
    vars: {
      brandPrimary: "#fb7185",
      brandSecondary: "#0b1220",
      appBg: "#0a0610",
      appSurface: "#150a14",
      appBorder: "#3b1931",
      appText: "#f1f5f9",
      appMuted: "#cbd5e1",
    },
  },
  {
    id: "play-05",
    mode: "play",
    label: "Royal Purple",
    description: "Deep violet with bright purple accents.",
    colorScheme: "dark",
    vars: {
      brandPrimary: "#8b5cf6",
      brandSecondary: "#0b1220",
      appBg: "#060513",
      appSurface: "#0d0b22",
      appBorder: "#2a1f55",
      appText: "#f1f5f9",
      appMuted: "#c4b5fd",
    },
  },
  {
    id: "play-06",
    mode: "play",
    label: "Arcade Day",
    description: "Bright slate base with energetic cyan accents.",
    colorScheme: "light",
    vars: {
      brandPrimary: "#22d3ee",
      brandSecondary: "#ffffff",
      appBg: "#f8fafc",
      appSurface: "#ffffff",
      appBorder: "#e2e8f0",
      appText: "#0f172a",
      appMuted: "#475569",
    },
  },
  {
    id: "play-07",
    mode: "play",
    label: "Bubblegum",
    description: "Playful magenta accents on a clean light base.",
    colorScheme: "light",
    vars: {
      brandPrimary: "#d946ef",
      brandSecondary: "#ffffff",
      appBg: "#f8fafc",
      appSurface: "#ffffff",
      appBorder: "#e2e8f0",
      appText: "#0f172a",
      appMuted: "#475569",
    },
  },
  {
    id: "play-08",
    mode: "play",
    label: "Lime Soda",
    description: "Vivid lime accent with slate-on-white surfaces.",
    colorScheme: "light",
    vars: {
      brandPrimary: "#84cc16",
      brandSecondary: "#ffffff",
      appBg: "#f8fafc",
      appSurface: "#ffffff",
      appBorder: "#e2e8f0",
      appText: "#0f172a",
      appMuted: "#475569",
    },
  },
  {
    id: "play-09",
    mode: "play",
    label: "Gold Loot (Light)",
    description: "Gold highlights with crisp, bright surfaces.",
    colorScheme: "light",
    vars: {
      brandPrimary: "#f59e0b",
      brandSecondary: "#ffffff",
      appBg: "#f8fafc",
      appSurface: "#ffffff",
      appBorder: "#e2e8f0",
      appText: "#0f172a",
      appMuted: "#475569",
    },
  },
  {
    id: "play-10",
    mode: "play",
    label: "Mint Chill (Light)",
    description: "Fresh mint accent with bright UI chrome.",
    colorScheme: "light",
    vars: {
      brandPrimary: "#34d399",
      brandSecondary: "#ffffff",
      appBg: "#f8fafc",
      appSurface: "#ffffff",
      appBorder: "#e2e8f0",
      appText: "#0f172a",
      appMuted: "#475569",
    },
  },
];

export function getThemePreset(mode: UIMode | undefined, themeId: string | undefined) {
  const m = mode ?? "work";
  const preferred = themeId ?? (m === "play" ? "play-01" : "work-01");
  return (
    THEME_PRESETS.find((t) => t.id === preferred) ??
    THEME_PRESETS.find((t) => t.mode === m) ??
    THEME_PRESETS[0]
  );
}

export const COLOR_PALETTES = {
  primary: [
    "#4f46e5",
    "#2563eb",
    "#0ea5e9",
    "#14b8a6",
    "#22c55e",
    "#a3e635",
    "#f59e0b",
    "#fbbf24",
    "#ef4444",
    "#e11d48",
    "#d946ef",
    "#8b5cf6",
    "#60a5fa",
    "#22d3ee",
    "#34d399",
    "#cbd5e1",
  ],
  background: [
    // dark
    "#020617",
    "#050817",
    "#030a18",
    "#041221",
    "#07051a",
    "#0a0610",
    "#0a0a06",
    "#05130f",
    // light
    "#f8fafc",
    "#f1f5f9",
    "#eef2ff",
    "#ecfeff",
    "#f0fdf4",
    "#fffbeb",
  ],
  surface: [
    // dark
    "#0b1220",
    "#071428",
    "#071a2c",
    "#0d0b22",
    "#120b24",
    "#150a14",
    "#14120a",
    "#072018",
    // light
    "#ffffff",
    "#f8fafc",
    "#f1f5f9",
  ],
  border: [
    // dark
    "#1f2937",
    "#1e293b",
    "#243041",
    "#23304a",
    "#1f3b57",
    "#2a1f55",
    "#334155",
    // light
    "#e2e8f0",
    "#cbd5e1",
  ],
  text: [
    // light text on dark
    "#f1f5f9",
    "#e2e8f0",
    "#cbd5e1",
    // dark text on light
    "#0f172a",
    "#1f2937",
  ],
  muted: [
    // dark mode muted
    "#94a3b8",
    "#93a4b8",
    "#a5b4fc",
    "#c4b5fd",
    "#7dd3fc",
    "#93c5fd",
    "#fde68a",
    "#86efac",
    // light mode muted
    "#475569",
    "#64748b",
  ],
};
