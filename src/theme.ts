// Catppuccin Mocha color palette
const catppuccin_mocha = {
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
};

// Semantic color mappings for the UI
export const theme = {
	// Background colors
	bg: catppuccin_mocha.base,
	bgDark: catppuccin_mocha.mantle,
	bgDarker: catppuccin_mocha.crust,
	bgHighlight: catppuccin_mocha.surface0,

	// Text colors
	text: catppuccin_mocha.text,
	textMuted: catppuccin_mocha.subtext0,
	textDim: catppuccin_mocha.overlay1,

	// UI element colors
	border: catppuccin_mocha.surface1,
	borderFocus: catppuccin_mocha.lavender,

	// Status colors
	success: catppuccin_mocha.green,
	warning: catppuccin_mocha.yellow,
	error: catppuccin_mocha.red,
	info: catppuccin_mocha.blue,

	// Accent colors for different elements
	accent: catppuccin_mocha.mauve,
	accentAlt: catppuccin_mocha.lavender,
	selected: catppuccin_mocha.green,
	cursor: catppuccin_mocha.rosewater,

	// Type badges
	github: catppuccin_mocha.peach,
	gitlab: catppuccin_mocha.flamingo,
	sourcehut: catppuccin_mocha.teal,
	path: catppuccin_mocha.sky,
	git: catppuccin_mocha.pink,
	other: catppuccin_mocha.overlay1,

	// Misc
	key: catppuccin_mocha.lavender,
	sha: catppuccin_mocha.peach,
} as const;
