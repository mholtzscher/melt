import type { ScrollBoxRenderable } from "@opentui/core";
import { useKeyboard } from "@opentui/solid";
import { batch, createEffect, createResource, createSignal, For, on, Show } from "solid-js";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { HelpBar } from "../components/HelpBar";
import { shortcuts } from "../config/shortcuts";
import { useScrollSync } from "../hooks/useScrollSync";
import { runEffectEither } from "../runtime";
import { githubService } from "../services/github";
import type { FlakeStore } from "../stores/flakeStore";
import { theme } from "../theme";
import type { FlakeInput, GitHubCommit } from "../types";

export interface ChangelogViewProps {
	store: FlakeStore;
	input: FlakeInput;
}

interface CommitRowProps {
	commit: GitHubCommit;
	isCursor: boolean;
	isLocked: boolean;
}

function CommitRow(props: CommitRowProps) {
	return (
		<box flexDirection="row" backgroundColor={props.isCursor ? theme.bgHighlight : undefined}>
			<box width={3}>
				<text fg={theme.warning}>{props.isLocked ? "\u{1F512}" : "  "}</text>
			</box>

			<box width={9}>
				<text fg={props.isLocked ? theme.warning : theme.sha}>{props.commit.shortSha}</text>
			</box>

			<box width={16}>
				<text fg={theme.info}>
					{props.commit.author.length > 14
						? `${props.commit.author.substring(0, 14)}..`
						: props.commit.author.padEnd(14)}
				</text>
			</box>

			<box width={10}>
				<text fg={theme.textDim}>{props.commit.date.padEnd(8)}</text>
			</box>

			<text
				fg={props.isCursor ? theme.cursor : props.isLocked ? theme.warning : theme.text}
				attributes={props.isCursor || props.isLocked ? 1 : 0}
			>
				{props.commit.message.length > 55 ? `${props.commit.message.substring(0, 55)}...` : props.commit.message}
			</text>
		</box>
	);
}

interface CommitStatsProps {
	lockedIndex: number;
	totalCommits: number;
}

function CommitStats(props: CommitStatsProps) {
	return (
		<box flexDirection="row" marginLeft={2}>
			<text fg={theme.success}>+{props.lockedIndex} new</text>
			<text fg={theme.warning}> {"\u{1F512}"} </text>
			<text fg={theme.textMuted}>{Math.max(0, props.totalCommits - props.lockedIndex - 1)} older</text>
		</box>
	);
}

function ChangelogLoading() {
	return (
		<box flexDirection="column" flexGrow={1} justifyContent="center" alignItems="center">
			<box flexDirection="row">
				<spinner name="dots" color={theme.accent} />
				<text fg={theme.text}> Loading commits...</text>
			</box>
		</box>
	);
}

export function ChangelogView(props: ChangelogViewProps) {
	const { actions } = props.store;
	let scrollBoxRef: ScrollBoxRenderable | undefined;

	// Use createResource for async data fetching with automatic loading state
	const [changelogData] = createResource(
		() => props.input,
		async (input) => {
			const result = await runEffectEither(githubService.getChangelog(input));
			if (result._tag === "Left") {
				return { commits: [], lockedIndex: 0 };
			}
			return result.right;
		},
	);

	// Derived state from resource
	const commits = () => changelogData()?.commits ?? [];
	const lockedIndex = () => changelogData()?.lockedIndex ?? 0;

	// Local UI state
	const [cursorIndex, setCursorIndex] = createSignal(0);
	const [showConfirm, setShowConfirm] = createSignal(false);
	const [confirmCommit, setConfirmCommit] = createSignal<GitHubCommit | undefined>();

	// Initialize cursor position when data loads
	createEffect(
		on(
			() => changelogData(),
			(data) => {
				if (data) {
					setCursorIndex(data.lockedIndex);
				}
			},
		),
	);

	// Scroll sync with explicit dependency tracking
	useScrollSync(cursorIndex, () => scrollBoxRef);

	function moveCursor(delta: number) {
		const len = commits().length;
		if (len === 0) return;
		setCursorIndex((prev) => {
			const next = prev + delta;
			if (next < 0) return 0;
			if (next >= len) return len - 1;
			return next;
		});
	}

	function showConfirmDialog() {
		const commit = commits()[cursorIndex()];
		if (commit) {
			batch(() => {
				setConfirmCommit(commit);
				setShowConfirm(true);
			});
		}
	}

	function hideConfirmDialog() {
		batch(() => {
			setShowConfirm(false);
			setConfirmCommit(undefined);
		});
	}

	async function handleConfirm() {
		const commit = confirmCommit();
		const { owner, repo } = props.input;
		if (!commit || !owner || !repo) return;
		hideConfirmDialog();
		const success = await actions.lockToCommit(props.input.name, commit.sha, owner, repo);
		if (success) {
			actions.closeChangelog();
			actions.refresh();
		}
	}

	useKeyboard((e) => {
		if (e.eventType === "release") return;

		if (showConfirm()) {
			switch (e.name) {
				case "y":
					handleConfirm();
					break;
				case "n":
				case "escape":
				case "q":
					hideConfirmDialog();
					break;
			}
			return;
		}

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
				showConfirmDialog();
				break;
			case "escape":
			case "q":
				actions.closeChangelog();
				break;
		}
	});

	return (
		<box flexDirection="column" flexGrow={1}>
			<box
				flexGrow={1}
				flexShrink={1}
				borderStyle="rounded"
				borderColor={theme.border}
				title={`${props.input.name} (${props.input.url})`}
			>
				<Show when={changelogData.loading}>
					<ChangelogLoading />
				</Show>

				<Show when={!changelogData.loading}>
					<Show
						when={commits().length > 0}
						fallback={
							<box paddingLeft={1} flexGrow={1}>
								<text fg={theme.success}>Already up to date!</text>
							</box>
						}
					>
						<scrollbox ref={scrollBoxRef} flexGrow={1} paddingLeft={1} paddingRight={1} overflow="hidden">
							<box flexDirection="column">
								<For each={commits()}>
									{(commit, index) => (
										<CommitRow
											commit={commit}
											isCursor={cursorIndex() === index()}
											isLocked={commit.isLocked === true}
										/>
									)}
								</For>
							</box>
						</scrollbox>
					</Show>
				</Show>
			</box>

			<HelpBar shortcuts={shortcuts.changelog}>
				<Show when={!changelogData.loading && commits().length > 0}>
					<CommitStats lockedIndex={lockedIndex()} totalCommits={commits().length} />
				</Show>
			</HelpBar>

			<ConfirmDialog visible={showConfirm()} inputName={props.input.name} commit={confirmCommit()} />
		</box>
	);
}
