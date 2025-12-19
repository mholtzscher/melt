import { useRenderer } from "@opentui/solid";
import { createSignal, onMount, Show } from "solid-js";
import { useFlakeActions } from "./hooks/useFlakeActions";
import type { FlakeData } from "./services/flake";
import { theme } from "./theme";
import type { FlakeInput, UpdateStatus } from "./types";
import { ChangelogView } from "./views/ChangelogView";
import { ListView } from "./views/ListView";

export interface AppProps {
	initialFlake: FlakeData;
}

export function App(props: AppProps) {
	const renderer = useRenderer();

	// Flake data state
	const [inputs, setInputs] = createSignal<FlakeInput[]>(
		props.initialFlake.inputs,
	);
	const flakePath = () => props.initialFlake.path;

	// Update statuses state
	const [updateStatuses, setUpdateStatuses] = createSignal<
		Map<string, UpdateStatus>
	>(new Map());

	// UI state
	const [loading, setLoading] = createSignal(false);
	const [statusMessage, setStatusMessage] = createSignal<string | undefined>();
	const [changelogInput, setChangelogInput] = createSignal<
		FlakeInput | undefined
	>();

	const flakeActions = useFlakeActions({
		flakePath,
		inputs,
		setInputs,
		setUpdateStatuses,
		setLoading,
		setStatusMessage,
	});

	function quit(code = 0) {
		renderer.destroy();
		process.exit(code);
	}

	function showChangelog(input: FlakeInput) {
		if (input.type !== "github") {
			setStatusMessage("Changelog only available for GitHub inputs");
			setTimeout(() => setStatusMessage(undefined), 2000);
			return;
		}

		setChangelogInput(input);
	}

	function closeChangelog() {
		setChangelogInput(undefined);
	}

	onMount(() => {
		flakeActions.checkUpdates(props.initialFlake.inputs);
	});

	return (
		<box flexDirection="column" flexGrow={1} backgroundColor={theme.bg}>
			<Show when={!changelogInput()}>
				<ListView
					inputs={inputs}
					updateStatuses={updateStatuses}
					statusMessage={statusMessage}
					loading={loading}
					onShowChangelog={showChangelog}
					onRefresh={flakeActions.refresh}
					onUpdateSelected={flakeActions.updateSelected}
					onUpdateAll={flakeActions.updateAll}
					onQuit={quit}
				/>
			</Show>
			<Show when={changelogInput()}>
				{(input: () => FlakeInput) => (
					<ChangelogView
						input={input()}
						onBack={closeChangelog}
						onLockToCommit={flakeActions.lockToCommit}
					/>
				)}
			</Show>
		</box>
	);
}
