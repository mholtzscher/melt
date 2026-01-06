import type { ScrollBoxRenderable } from "@opentui/core";
import { useKeyboard } from "@opentui/solid";
import { createMemo, createSignal, For, Show } from "solid-js";
import { HelpBar } from "../components/HelpBar";
import { shortcuts } from "../config/shortcuts";
import { useScrollSync } from "../hooks/useScrollSync";
import { timeService } from "../services/time";
import type { FlakeStore } from "../stores/flakeStore";
import { theme } from "../theme";
import type { FlakeInput, UpdateStatus } from "../types";

const columns = {
	checkbox: 5,
	name: 35,
	type: 12,
	rev: 10,
	updated: 14,
	typePadding: 10,
} as const;

export interface ListViewProps {
	store: FlakeStore;
	onQuit: () => void;
}

interface StatusCellProps {
	status: UpdateStatus | undefined;
}

function StatusCell(props: StatusCellProps) {
	return (
		<switch fallback={<text fg={theme.textDim}>ok</text>}>
			<match when={!props.status}>
				<text fg={theme.textDim}>-</text>
			</match>
			<match when={props.status?.loading}>
				<spinner name="dots" color={theme.textDim} />
			</match>
			<match when={props.status?.error}>
				<text fg={theme.warning}>?</text>
			</match>
			<match when={props.status?.commitsBehind}>
				<text fg={theme.success}>+{props.status?.commitsBehind}</text>
			</match>
		</switch>
	);
}

interface FlakeRowProps {
	input: FlakeInput;
	index: number;
	isSelected: boolean;
	isCursor: boolean;
	status: UpdateStatus | undefined;
}

function FlakeRow(props: FlakeRowProps) {
	const badgeColor = getTypeBadgeColor(props.input.type);

	return (
		<box flexDirection="row" backgroundColor={props.isCursor ? theme.bgHighlight : undefined}>
			<box width={columns.checkbox}>
				<text fg={props.isSelected ? theme.selected : theme.textDim} attributes={props.isSelected ? 1 : 0}>
					{props.isSelected ? "[x] " : "[ ] "}
				</text>
			</box>

			<box width={columns.name}>
				<text fg={props.isCursor ? theme.cursor : theme.text} attributes={props.isCursor ? 1 : 0}>
					{props.input.name}
				</text>
			</box>

			<box width={columns.type}>
				<text fg={badgeColor}>{props.input.type.padEnd(columns.typePadding)}</text>
			</box>

			<box width={columns.rev}>
				<text fg={theme.accent}>{props.input.shortRev}</text>
			</box>

			<box width={columns.updated}>
				<text fg={theme.textMuted}>{timeService.formatRelativeTime(props.input.lastModified)}</text>
			</box>

			<StatusCell status={props.status} />
		</box>
	);
}

function TableHeader() {
	return (
		<box flexDirection="row">
			<box width={columns.checkbox}>
				<text fg={theme.textDim}> </text>
			</box>
			<box width={columns.name}>
				<text fg={theme.textDim}>NAME</text>
			</box>
			<box width={columns.type}>
				<text fg={theme.textDim}>TYPE</text>
			</box>
			<box width={columns.rev}>
				<text fg={theme.textDim}>REV</text>
			</box>
			<box width={columns.updated}>
				<text fg={theme.textDim}>UPDATED</text>
			</box>
			<text fg={theme.textDim}>STATUS</text>
		</box>
	);
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
	const { state, actions } = props.store;
	let scrollBoxRef: ScrollBoxRenderable | undefined;

	const [cursorIndex, setCursorIndex] = createSignal(0);
	const [selectedIndices, setSelectedIndices] = createSignal<Set<number>>(new Set());

	// Scroll sync with explicit dependency tracking
	useScrollSync(cursorIndex, () => scrollBoxRef);

	// Memoized derived values
	const currentInput = createMemo(() => state.inputs[cursorIndex()]);

	const selectedNames = createMemo(() =>
		Array.from(selectedIndices())
			.map((i) => state.inputs[i]?.name)
			.filter((n): n is string => !!n),
	);

	function moveCursor(delta: number) {
		const len = state.inputs.length;
		if (len === 0) return;
		const prev = cursorIndex();
		const next = prev + delta;
		if (next < 0) setCursorIndex(0);
		else if (next >= len) setCursorIndex(len - 1);
		else setCursorIndex(next);
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

	function handleQuit() {
		if (selectedIndices().size > 0) {
			clearSelection();
		} else {
			props.onQuit();
		}
	}

	useKeyboard((e) => {
		if (e.eventType === "release") return;

		const keyActions: Record<string, () => void> = {
			j: () => moveCursor(1),
			down: () => moveCursor(1),
			k: () => moveCursor(-1),
			up: () => moveCursor(-1),
			space: toggleSelection,
			u: () => {
				if (e.shift) {
					actions.updateAll();
				} else {
					const names = selectedNames();
					if (names.length > 0) {
						actions.updateSelected(names);
						clearSelection();
					}
				}
			},
			c: () => {
				const input = currentInput();
				if (input) actions.showChangelog(input);
			},
			r: () => actions.refresh(),
			escape: handleQuit,
			q: handleQuit,
		};

		const action = keyActions[e.name];
		if (action) action();
	});

	return (
		<box flexDirection="column" flexGrow={1}>
			<box
				flexDirection="column"
				flexGrow={1}
				flexShrink={1}
				borderStyle="rounded"
				borderColor={theme.border}
				title={state.path}
			>
				<box flexDirection="column" paddingLeft={1} paddingRight={1} flexShrink={0}>
					<TableHeader />
				</box>

				<scrollbox ref={scrollBoxRef} flexGrow={1} paddingLeft={1} paddingRight={1} overflow="hidden">
					<box flexDirection="column">
						<For each={state.inputs}>
							{(input, index) => (
								<FlakeRow
									input={input}
									index={index()}
									isSelected={selectedIndices().has(index())}
									isCursor={cursorIndex() === index()}
									status={state.updateStatuses[input.name]}
								/>
							)}
						</For>

						<Show when={state.inputs.length === 0}>
							<box justifyContent="center" paddingTop={2} paddingBottom={2}>
								<text fg={theme.textMuted}>No flake inputs found</text>
							</box>
						</Show>
					</box>
				</scrollbox>
			</box>

			<HelpBar shortcuts={shortcuts.list}>
				<Show when={selectedIndices().size > 0}>
					<box marginLeft={2}>
						<text fg={theme.selected}>{selectedIndices().size} selected</text>
					</box>
				</Show>
			</HelpBar>
		</box>
	);
}
