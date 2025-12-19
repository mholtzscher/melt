import { useKeyboard } from "@opentui/solid";
import type { Accessor } from "solid-js";
import { createSignal, For, Show } from "solid-js";
import { HelpBar } from "../components/HelpBar";
import { shortcuts } from "../config/shortcuts";
import { timeService } from "../services/time";
import { mocha, theme } from "../theme";
import type { FlakeInput, UpdateStatus } from "../types";

export interface ListViewProps {
	inputs: Accessor<FlakeInput[]>;
	updateStatuses: Accessor<Map<string, UpdateStatus>>;
	statusMessage: Accessor<string | undefined>;
	loading: Accessor<boolean>;
	onShowChangelog: (input: FlakeInput) => void;
	onRefresh: () => void;
	onUpdateSelected: (names: string[]) => void;
	onUpdateAll: () => void;
	onQuit: () => void;
}

function getTypeBadgeColor(type: FlakeInput["type"]): string {
	switch (type) {
		case "github":
			return theme.github;
		case "gitlab":
			return theme.gitlab;
		case "sourcehut":
			return theme.sourcehut;
		case "path":
			return theme.path;
		case "git":
			return theme.git;
		default:
			return theme.other;
	}
}

export function ListView(props: ListViewProps) {
	const [cursorIndex, setCursorIndex] = createSignal(0);
	const [selectedIndices, setSelectedIndices] = createSignal<Set<number>>(
		new Set(),
	);

	function moveCursor(delta: number) {
		const len = props.inputs().length;
		if (len === 0) return;
		const prev = cursorIndex();
		const next = prev + delta;
		if (next < 0) setCursorIndex(0);
		else if (next >= len) setCursorIndex(len - 1);
		else setCursorIndex(next);
	}

	function getCurrentInput() {
		return props.inputs()[cursorIndex()];
	}

	function getSelectedNames(): string[] {
		return Array.from(selectedIndices())
			.map((i) => props.inputs()[i]?.name)
			.filter((n): n is string => !!n);
	}

	function toggleSelection() {
		const idx = cursorIndex();
		const next = new Set(selectedIndices());
		if (next.has(idx)) {
			next.delete(idx);
		} else {
			next.add(idx);
		}
		setSelectedIndices(next);
	}

	function clearSelection() {
		setSelectedIndices(new Set<number>());
	}

	useKeyboard((e) => {
		if (e.eventType === "release") return;

		switch (e.name) {
			case "j":
			case "down":
				moveCursor(1);
				break;
			case "k":
			case "up":
				moveCursor(-1);
				break;
			case "space":
				toggleSelection();
				break;
			case "u":
				if (e.shift) {
					props.onUpdateAll();
				} else {
					const names = getSelectedNames();
					if (names.length > 0) {
						props.onUpdateSelected(names);
						clearSelection();
					}
				}
				break;
			case "c": {
				const input = getCurrentInput();
				if (input) props.onShowChangelog(input);
				break;
			}
			case "r":
				props.onRefresh();
				break;
			case "escape":
			case "q":
				if (selectedIndices().size > 0) {
					clearSelection();
				} else {
					props.onQuit();
				}
				break;
		}
	});

	return (
		<box flexDirection="column" flexGrow={1}>
			<box flexDirection="column" flexGrow={1} paddingLeft={1} paddingRight={1}>
				<box flexDirection="row">
					<box width={5}>
						<text fg={theme.textDim}> </text>
					</box>
					<box width={35}>
						<text fg={theme.textDim}>NAME</text>
					</box>
					<box width={12}>
						<text fg={theme.textDim}>TYPE</text>
					</box>
					<box width={10}>
						<text fg={theme.textDim}>REV</text>
					</box>
					<box width={14}>
						<text fg={theme.textDim}>UPDATED</text>
					</box>
					<text fg={theme.textDim}>STATUS</text>
				</box>

				<For each={props.inputs()}>
					{(input, index) => {
						const isSelected = () => selectedIndices().has(index());
						const isCursor = () => cursorIndex() === index();
						const badgeColor = getTypeBadgeColor(input.type);

						return (
							<box
								flexDirection="row"
								backgroundColor={isCursor() ? theme.bgHighlight : undefined}
							>
								<box width={5}>
									<text
										fg={isSelected() ? theme.selected : theme.textDim}
										attributes={isSelected() ? 1 : 0}
									>
										{isSelected() ? "[x] " : "[ ] "}
									</text>
								</box>

								<box width={35}>
									<text
										fg={isCursor() ? theme.cursor : theme.text}
										attributes={isCursor() ? 1 : 0}
									>
										{input.name}
									</text>
								</box>

								<box width={12}>
									<text fg={badgeColor}>{input.type.padEnd(10)}</text>
								</box>

								<box width={10}>
									<text fg={theme.accent}>{input.shortRev}</text>
								</box>

								<box width={14}>
									<text fg={theme.textMuted}>
										{timeService.formatRelativeTime(input.lastModified)}
									</text>
								</box>

								{(() => {
									const status = props.updateStatuses().get(input.name);
									if (!status) return <text fg={theme.textDim}>-</text>;
									if (status.loading) {
										return <spinner name="dots" color={theme.textDim} />;
									}
									if (status.error) {
										return <text fg={mocha.yellow}>?</text>;
									}
									if (status.hasUpdate) {
										return (
											<text fg={mocha.green}>+{status.commitsBehind}</text>
										);
									}
									return <text fg={theme.textDim}>ok</text>;
								})()}
							</box>
						);
					}}
				</For>

				<Show when={props.inputs().length === 0}>
					<box justifyContent="center" paddingTop={2} paddingBottom={2}>
						<text fg={theme.textMuted}>No flake inputs found</text>
					</box>
				</Show>
			</box>

			<HelpBar
				statusMessage={props.statusMessage}
				loading={props.loading}
				selectedCount={() => selectedIndices().size}
				shortcuts={shortcuts.list}
			/>
		</box>
	);
}
