import { createContext, type JSX, onMount, useContext } from "solid-js";
import { createStore, produce } from "solid-js/store";
import {
	getFlakeMetadata,
	hasFlakeNix,
	lockInputToRev,
	resolveFlakePath,
	updateAll,
	updateInputs,
} from "../lib/flake";
import { checkForUpdates, hasGitHubToken } from "../lib/github";
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

	// Guard to prevent concurrent update checks
	let isCheckingUpdates = false;

	// Check for updates on all inputs
	async function checkUpdates(inputsList?: FlakeInput[]) {
		if (isCheckingUpdates) return;
		isCheckingUpdates = true;

		const toCheck = inputsList || state.inputs;
		const tokenMsg = hasGitHubToken()
			? ""
			: " (set GITHUB_TOKEN for higher rate limits)";
		setState("statusMessage", `Checking for updates...${tokenMsg}`);

		try {
			await checkForUpdates(toCheck, (name, status) => {
				setState(
					produce((s) => {
						const newMap = new Map(s.updateStatuses);
						newMap.set(name, status);
						s.updateStatuses = newMap;
					}),
				);
			});
			setState("statusMessage", undefined);
		} catch (err) {
			const errorMsg = err instanceof Error ? err.message : String(err);
			if (errorMsg.includes("rate limit")) {
				setState("statusMessage", `${errorMsg} - set GITHUB_TOKEN env var`);
			} else {
				setState("statusMessage", `Error checking updates: ${errorMsg}`);
			}
			setTimeout(() => setState("statusMessage", undefined), 5000);
		} finally {
			isCheckingUpdates = false;
		}
	}

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
			checkUpdates(metadata.inputs);
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

		async refresh() {
			setState("statusMessage", "Refreshing...");
			try {
				const metadata = await getFlakeMetadata(flakePath);
				setState("inputs", metadata.inputs);
				setState("description", metadata.description);

				// Re-check for updates after refresh
				await checkUpdates(metadata.inputs);
			} catch (err) {
				setState(
					"statusMessage",
					`Error: ${err instanceof Error ? err.message : err}`,
				);
				setTimeout(() => setState("statusMessage", undefined), 3000);
			}
		},

		async updateSelected() {
			const selected = state.selectedIndices;
			if (selected.size === 0) {
				setState("statusMessage", "No inputs selected");
				setTimeout(() => setState("statusMessage", undefined), 2000);
				return;
			}

			const names = Array.from(selected)
				.map((i) => state.inputs[i]?.name)
				.filter((n): n is string => !!n);
			setState("statusMessage", `Updating ${names.join(", ")}...`);
			setState("loading", true);

			const result = await updateInputs(names, flakePath);
			setState("loading", false);

			if (result.success) {
				setState("selectedIndices", new Set<number>());
				await actions.refresh();
				setState("statusMessage", `Updated ${names.length} input(s)`);
			} else {
				setState("statusMessage", `Error: ${result.output}`);
			}

			setTimeout(() => setState("statusMessage", undefined), 3000);
		},

		async updateAll() {
			setState("statusMessage", "Updating all inputs...");
			setState("loading", true);

			const result = await updateAll(flakePath);
			setState("loading", false);

			if (result.success) {
				setState("selectedIndices", new Set<number>());
				await actions.refresh();
				setState("statusMessage", "All inputs updated");
			} else {
				setState("statusMessage", `Error: ${result.output}`);
			}

			setTimeout(() => setState("statusMessage", undefined), 3000);
		},

		async lockToCommit(
			inputName: string,
			sha: string,
			owner: string,
			repo: string,
		): Promise<boolean> {
			setState(
				"statusMessage",
				`Locking ${inputName} to ${sha.substring(0, 7)}...`,
			);

			const result = await lockInputToRev(
				inputName,
				sha,
				owner,
				repo,
				flakePath,
			);

			if (result.success) {
				setState(
					"statusMessage",
					`Locked ${inputName} to ${sha.substring(0, 7)}`,
				);
				await actions.refresh();
				setTimeout(() => setState("statusMessage", undefined), 3000);
				return true;
			} else {
				setState("statusMessage", `Error: ${result.output}`);
				setTimeout(() => setState("statusMessage", undefined), 3000);
				return false;
			}
		},

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
