import { useKeyboard, useRenderer } from "@opentui/solid";
import { createResource, Match, onMount, Show, Switch } from "solid-js";
import { runEffect } from "./runtime";
import type { FlakeData } from "./services/flake";
import { flakeService } from "./services/flake";
import { mountToaster } from "./services/toast";
import { createFlakeStore } from "./stores/flakeStore";
import { theme } from "./theme";
import type { FlakeInput } from "./types";
import { ChangelogView } from "./views/ChangelogView";
import { ListView } from "./views/ListView";

export interface AppProps {
	flakePath?: string;
}

function toErrorMessage(err: unknown): string {
	return err instanceof Error ? err.message : String(err);
}

function LoadingScreen(props: { onQuit: () => void }) {
	useKeyboard((e) => {
		if (e.eventType === "release") return;
		if (e.name === "escape" || e.name === "q") {
			props.onQuit();
		}
	});

	return (
		<box flexDirection="column" flexGrow={1} justifyContent="center" alignItems="center" backgroundColor={theme.bg}>
			<box flexDirection="column" alignItems="center">
				<box flexDirection="row">
					<spinner name="dots" color={theme.accent} />
					<text fg={theme.text}> Loading flake...</text>
				</box>
				<text fg={theme.textDim}>Press q or esc to quit</text>
			</box>
		</box>
	);
}

function ErrorScreen(props: { error: string; onQuit: () => void }) {
	useKeyboard((e) => {
		if (e.eventType === "release") return;
		props.onQuit();
	});

	return (
		<box flexDirection="column" flexGrow={1} justifyContent="center" alignItems="center" backgroundColor={theme.bg}>
			<text fg={theme.error}>Error: {props.error}</text>
			<text fg={theme.textDim}>Press any key to exit</text>
		</box>
	);
}

export function App(props: AppProps) {
	const renderer = useRenderer();

	onMount(() => {
		mountToaster(renderer);
	});

	const [flakeData] = createResource(
		() => props.flakePath ?? ".",
		async (flakePath) => {
			return runEffect(flakeService.load(flakePath));
		},
	);

	function quit(code = 0) {
		renderer.destroy();
		process.exit(code);
	}

	return (
		<Switch>
			<Match when={flakeData.state === "pending" || flakeData.state === "unresolved"}>
				<LoadingScreen onQuit={quit} />
			</Match>
			<Match when={flakeData.state === "errored"}>
				<ErrorScreen error={toErrorMessage(flakeData.error)} onQuit={() => quit(1)} />
			</Match>
			<Match when={flakeData.state === "ready" && flakeData()}>
				{(data: () => FlakeData) => <MainView flake={data()} onQuit={quit} />}
			</Match>
		</Switch>
	);
}

interface MainViewProps {
	flake: FlakeData;
	onQuit: (code?: number) => void;
}

function MainView(props: MainViewProps) {
	const { state, actions } = createFlakeStore(props.flake);

	onMount(() => {
		actions.checkUpdates();
	});

	return (
		<box flexDirection="column" flexGrow={1} backgroundColor={theme.bg}>
			<Show when={!state.changelogInput}>
				<ListView store={{ state, actions }} onQuit={props.onQuit} />
			</Show>
			<Show when={state.changelogInput}>
				{(input: () => FlakeInput) => <ChangelogView store={{ state, actions }} input={input()} />}
			</Show>
		</box>
	);
}
