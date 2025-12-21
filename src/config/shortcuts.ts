export interface HelpItem {
	key: string;
	description: string;
}

type ViewType = "list" | "changelog" | "updating";

export const shortcuts: Record<ViewType, HelpItem[]> = {
	list: [
		{ key: "j/k", description: "nav" },
		{ key: "space", description: "select" },
		{ key: "u", description: "update" },
		{ key: "U", description: "all" },
		{ key: "c", description: "changelog" },
		{ key: "r", description: "refresh" },
		{ key: "q/esc", description: "quit" },
	],
	changelog: [
		{ key: "j/k", description: "nav" },
		{ key: "space", description: "lock" },
		{ key: "q/esc", description: "back" },
	],
	updating: [],
};

export const confirmShortcuts: HelpItem[] = [
	{ key: "y", description: "confirm" },
	{ key: "n/q", description: "cancel" },
];
