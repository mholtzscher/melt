import { useRenderer } from "@opentui/solid";
import { onMount, Show } from "solid-js";
import type { FlakeData } from "./services/flake";
import { createFlakeStore } from "./stores/flakeStore";
import { theme } from "./theme";
import type { FlakeInput } from "./types";
import { ChangelogView } from "./views/ChangelogView";
import { ListView } from "./views/ListView";

export interface AppProps {
	initialFlake: FlakeData;
}

export function App(props: AppProps) {
	const renderer = useRenderer();
	const { state, actions } = createFlakeStore(props.initialFlake);

	function quit(code = 0) {
		renderer.destroy();
		process.exit(code);
	}

	onMount(() => {
		actions.checkUpdates();
	});

	return (
		<box flexDirection="column" flexGrow={1} backgroundColor={theme.bg}>
			<Show when={!state.changelogInput}>
				<ListView store={{ state, actions }} onQuit={quit} />
			</Show>
			<Show when={state.changelogInput}>
				{(input: () => FlakeInput) => (
					<ChangelogView store={{ state, actions }} input={input()} />
				)}
			</Show>
		</box>
	);
}
