import { createContext, type JSX, onMount, useContext } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { createFlakeLogic } from "../hooks/createFlakeLogic";
import type {
	AppView,
	FlakeInput,
	FlakeMetadata,
	UpdateStatus,
} from "../lib/types";

export interface AppState {
	// View state
	view: AppView;
	loading: boolean;
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

export interface AppProviderProps {
	flake: FlakeMetadata;
	children: JSX.Element;
}

export function AppProvider(props: AppProviderProps) {
	const [state, setState] = createStore<AppState>({
		view: "list",
		loading: false,
		inputs: props.flake.inputs,
		description: props.flake.description,
		cursorIndex: 0,
		selectedIndices: new Set(),
		updateStatuses: new Map(),
	});

	const flakeLogic = createFlakeLogic(state, setState, props.flake.path);

	// Check for updates in background on mount
	onMount(() => {
		flakeLogic.checkUpdates(props.flake.inputs);
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
			return props.flake.path;
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
