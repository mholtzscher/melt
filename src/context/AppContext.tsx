import { createContext, type JSX, onMount, useContext } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { createFlakeLogic } from "../hooks/createFlakeLogic";
import { getFlakeMetadata, hasFlakeNix, resolveFlakePath } from "../lib/flake";
import type { AppView, FlakeInput, UpdateStatus } from "../lib/types";

// Get the flake path from command line args or use current working directory
const flakePath = resolveFlakePath(process.argv[2] || process.cwd());

export interface AppState {
	// View state
	view: AppView;
	loading: boolean;
	error?: string;
	statusMessage?: string;

	// Flake data
	inputs: FlakeInput[];
	description?: string;

	// List navigation
	cursorIndex: number;
	selectedIndices: Set<number>;

	// Update status for each input
	updateStatuses: Map<string, UpdateStatus>;
}

export interface AppActions {
	// Navigation
	moveCursor: (delta: number) => void;
	toggleSelection: () => void;
	clearSelection: () => void;

	// View management
	setView: (view: AppView) => void;
	setError: (error: string) => void;
	setStatusMessage: (message: string | undefined) => void;

	// Data operations
	refresh: () => Promise<void>;
	updateSelected: () => Promise<void>;
	updateAll: () => Promise<void>;
	lockToCommit: (
		inputName: string,
		sha: string,
		owner: string,
		repo: string,
	) => Promise<boolean>;

	// Getters
	getCurrentInput: () => FlakeInput | undefined;
	getFlakePath: () => string;
}

type AppContextValue = [AppState, AppActions];

const AppContext = createContext<AppContextValue>();

export function AppProvider(props: { children: JSX.Element }) {
	const [state, setState] = createStore<AppState>({
		view: "list",
		loading: true,
		inputs: [],
		cursorIndex: 0,
		selectedIndices: new Set(),
		updateStatuses: new Map(),
	});

	const flakeLogic = createFlakeLogic(state, setState, flakePath);

	// Load flake data on mount
	onMount(async () => {
		try {
			const hasFlake = await hasFlakeNix(flakePath);
			if (!hasFlake) {
				setState("error", `No flake.nix found in ${flakePath}`);
				setState("view", "error");
				setState("loading", false);
				return;
			}

			const metadata = await getFlakeMetadata(flakePath);
			setState("inputs", metadata.inputs);
			setState("description", metadata.description);
			setState("loading", false);

			// Check for updates in background
			flakeLogic.checkUpdates(metadata.inputs);
		} catch (err) {
			setState("error", err instanceof Error ? err.message : String(err));
			setState("view", "error");
			setState("loading", false);
		}
	});

	const actions: AppActions = {
		moveCursor(delta: number) {
			const len = state.inputs.length;
			if (len === 0) return;
			setState("cursorIndex", (prev) => {
				const next = prev + delta;
				if (next < 0) return 0;
				if (next >= len) return len - 1;
				return next;
			});
		},

		toggleSelection() {
			const idx = state.cursorIndex;
			setState(
				produce((s) => {
					const next = new Set(s.selectedIndices);
					if (next.has(idx)) {
						next.delete(idx);
					} else {
						next.add(idx);
					}
					s.selectedIndices = next;
				}),
			);
		},

		clearSelection() {
			setState("selectedIndices", new Set<number>());
		},

		setView(view: AppView) {
			setState("view", view);
		},

		setError(error: string) {
			setState("error", error);
		},

		setStatusMessage(message: string | undefined) {
			setState("statusMessage", message);
		},

		refresh: flakeLogic.refresh,
		updateSelected: flakeLogic.updateSelected,
		updateAll: flakeLogic.updateAll,
		lockToCommit: flakeLogic.lockToCommit,

		getCurrentInput() {
			return state.inputs[state.cursorIndex];
		},

		getFlakePath() {
			return flakePath;
		},
	};

	return (
		<AppContext.Provider value={[state, actions]}>
			{props.children}
		</AppContext.Provider>
	);
}

export function useApp(): AppContextValue {
	const context = useContext(AppContext);
	if (!context) {
		throw new Error("useApp must be used within an AppProvider");
	}
	return context;
}
