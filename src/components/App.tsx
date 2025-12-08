import { useKeyboard, useRenderer } from "@opentui/solid";
import { type Accessor, Match, Show, Switch } from "solid-js";
import { useApp } from "../context/AppContext";
import { useChangelog } from "../context/ChangelogContext";
import { theme } from "../lib/theme";
import type { FlakeInput } from "../lib/types";
import { Changelog } from "./Changelog";
import { ConfirmDialog } from "./ConfirmDialog";
import { FlakeList } from "./FlakeList";
import { StatusBar } from "./StatusBar";

export function App() {
	const [appState, appActions] = useApp();
	const [changelogState, changelogActions] = useChangelog();
	const renderer = useRenderer();

	function quit(code = 0) {
		renderer.destroy();
		process.exit(code);
	}

	// Show changelog for current input
	async function showChangelog() {
		const input = appActions.getCurrentInput();
		if (!input) return;

		if (input.type !== "github") {
			appActions.setStatusMessage("Changelog only available for GitHub inputs");
			setTimeout(() => appActions.setStatusMessage(undefined), 2000);
			return;
		}

		appActions.setView("changelog");

		try {
			await changelogActions.open(input);
		} catch (err) {
			appActions.setStatusMessage(
				`Error loading changelog: ${err instanceof Error ? err.message : err}`,
			);
		}
	}

	// Handle locking to a commit
	async function handleLockToCommit() {
		const input = changelogState.input;
		const commit = changelogState.confirmCommit;
		if (!input || !commit || !input.owner || !input.repo) return;

		changelogActions.hideConfirmDialog();

		const success = await appActions.lockToCommit(
			input.name,
			commit.sha,
			input.owner,
			input.repo,
		);

		if (success) {
			appActions.setView("list");
			changelogActions.close();
		}
	}

	// Keyboard handling
	useKeyboard((e) => {
		if (e.eventType === "release") return;

		const currentView = appState.view;

		// Changelog view keys
		if (currentView === "changelog") {
			// Handle confirmation dialog keys first
			if (changelogState.showConfirm) {
				switch (e.name) {
					case "y":
						handleLockToCommit();
						break;
					case "n":
					case "escape":
					case "q":
						changelogActions.hideConfirmDialog();
						break;
				}
				return;
			}

			switch (e.name) {
				case "j":
				case "down":
					changelogActions.moveCursor(1);
					break;
				case "k":
				case "up":
					changelogActions.moveCursor(-1);
					break;
				case "space":
					changelogActions.showConfirmDialog();
					break;
				case "escape":
				case "q":
					appActions.setView("list");
					changelogActions.close();
					break;
			}
			return;
		}

		// List view keybindings
		switch (e.name) {
			case "j":
			case "down":
				appActions.moveCursor(1);
				break;
			case "k":
			case "up":
				appActions.moveCursor(-1);
				break;
			case "space":
				appActions.toggleSelection();
				break;
			case "u":
				if (e.shift) {
					appActions.updateAll();
				} else {
					appActions.updateSelected();
				}
				break;
			case "c":
				showChangelog();
				break;
			case "r":
				appActions.refresh();
				break;
			case "escape":
			case "q":
				// If selection exists, clear it; otherwise quit
				if (appState.selectedIndices.size > 0) {
					appActions.clearSelection();
				} else {
					quit(0);
				}
				break;
		}
	});

	return (
		<box flexDirection="column" flexGrow={1} backgroundColor={theme.bg}>
			<Switch>
				{/* Changelog view */}
				<Match when={appState.view === "changelog"}>
					<Show when={changelogState.input}>
						{(input: Accessor<FlakeInput>) => (
							<>
								<Changelog
									input={input()}
									commits={changelogState.commits}
									loading={changelogState.loading}
									cursorIndex={changelogState.cursorIndex}
									lockedIndex={changelogState.lockedIndex}
								/>
								<ConfirmDialog
									visible={changelogState.showConfirm}
									inputName={input().name}
									commit={changelogState.confirmCommit}
								/>
							</>
						)}
					</Show>
				</Match>

				{/* Main list view */}
				<Match when={appState.view === "list"}>
					<FlakeList
						inputs={appState.inputs}
						cursorIndex={appState.cursorIndex}
						selectedIndices={appState.selectedIndices}
						updateStatuses={appState.updateStatuses}
					/>
					<StatusBar
						statusMessage={appState.statusMessage}
						loading={appState.loading}
						selectedCount={appState.selectedIndices.size}
					/>
				</Match>
			</Switch>
		</box>
	);
}
