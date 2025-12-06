// Catppuccin Mocha color palette
export const mocha = {
  // Accent colors
  rosewater: "#f5e0dc",
  flamingo: "#f2cdcd",
  pink: "#f5c2e7",
  mauve: "#cba6f7",
  red: "#f38ba8",
  maroon: "#eba0ac",
  peach: "#fab387",
  yellow: "#f9e2af",
  green: "#a6e3a1",
  teal: "#94e2d5",
  sky: "#89dceb",
  sapphire: "#74c7ec",
  blue: "#89b4fa",
  lavender: "#b4befe",

  // Neutral colors
  text: "#cdd6f4",
  subtext1: "#bac2de",
  subtext0: "#a6adc8",
  overlay2: "#9399b2",
  overlay1: "#7f849c",
  overlay0: "#6c7086",
  surface2: "#585b70",
  surface1: "#45475a",
  surface0: "#313244",
  base: "#1e1e2e",
  mantle: "#181825",
  crust: "#11111b",
} as const;

// Semantic color mappings for the UI
export const theme = {
  // Background colors
  bg: mocha.base,
  bgDark: mocha.mantle,
  bgDarker: mocha.crust,
  bgHighlight: mocha.surface0,

  // Text colors
  text: mocha.text,
  textMuted: mocha.subtext0,
  textDim: mocha.overlay1,

  // UI element colors
  border: mocha.surface1,
  borderFocus: mocha.lavender,

  // Status colors
  success: mocha.green,
  warning: mocha.yellow,
  error: mocha.red,
  info: mocha.blue,

  // Accent colors for different elements
  accent: mocha.mauve,
  accentAlt: mocha.lavender,
  selected: mocha.green,
  cursor: mocha.rosewater,

  // Type badges
  github: mocha.peach,
  gitlab: mocha.flamingo,
  sourcehut: mocha.teal,
  path: mocha.sky,
  git: mocha.pink,
  other: mocha.overlay1,
} as const;
