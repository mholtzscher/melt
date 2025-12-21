import type { ScrollBoxRenderable } from "@opentui/core";
import { useKeyboard } from "@opentui/solid";
import { createEffect, createSignal, For, onMount, Show } from "solid-js";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { shortcuts } from "../config/shortcuts";
import { githubService } from "../services/github";
import type { FlakeStore } from "../stores/flakeStore";
import { theme } from "../theme";
import type { FlakeInput, GitHubCommit } from "../types";

export interface ChangelogViewProps {
	store: FlakeStore;
	input: FlakeInput;
}

export function ChangelogView(props: ChangelogViewProps) {
	const { actions } = props.store;
	let scrollBoxRef: ScrollBoxRenderable | undefined;

	const [commits, setCommits] = createSignal<GitHubCommit[]>([]);
	const [lockedIndex, setLockedIndex] = createSignal(0);
	const [cursorIndex, setCursorIndex] = createSignal(0);
	const [loading, setLoading] = createSignal(true);
	const [showConfirm, setShowConfirm] = createSignal(false);
	const [confirmCommit, setConfirmCommit] = createSignal<
		GitHubCommit | undefined
	>();

	createEffect(() => {
		const cursor = cursorIndex();
		if (scrollBoxRef) {
			scrollBoxRef.scrollTop = Math.max(0, cursor - 5);
		}
	});

	onMount(async () => {
		try {
			const result = await githubService.getChangelog(props.input);
			setCommits(result.commits);
			setLockedIndex(result.lockedIndex);
			setCursorIndex(result.lockedIndex);
		} catch (_err) {
			setCommits([]);
			setLockedIndex(0);
		} finally {
			setLoading(false);
		}
	});

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
			setConfirmCommit(commit);
			setShowConfirm(true);
		}
	}

	function hideConfirmDialog() {
		setShowConfirm(false);
		setConfirmCommit(undefined);
	}

	async function handleConfirm() {
		const commit = confirmCommit();
		const { owner, repo } = props.input;
		if (!commit || !owner || !repo) return;
		hideConfirmDialog();
		const success = await actions.lockToCommit(
			props.input.name,
			commit.sha,
			owner,
			repo,
		);
		if (success) {
			actions.closeChangelog();
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
				flexDirection="row"
				backgroundColor={theme.bgDark}
				paddingLeft={1}
				paddingRight={1}
				flexShrink={0}
				height={1}
			>
				<text fg={theme.accent} attributes={1}>
					Changelog: {props.input.name}
				</text>
				<text fg={theme.textDim}> ({props.input.url})</text>
			</box>

			<Show when={loading()}>
				<box paddingLeft={1} flexGrow={1}>
					<text fg={theme.warning}>Loading commits...</text>
				</box>
			</Show>

			<Show when={!loading()}>
				<Show
					when={commits().length > 0}
					fallback={
						<box paddingLeft={1} flexGrow={1}>
							<text fg={theme.success}>Already up to date!</text>
						</box>
					}
				>
					<scrollbox
						ref={scrollBoxRef}
						flexGrow={1}
						flexShrink={1}
						paddingLeft={1}
						paddingRight={1}
						overflow="hidden"
					>
						<box flexDirection="column">
							<For each={commits()}>
								{(commit, index) => {
									const isCursor = () => cursorIndex() === index();
									const isLocked = () => commit.isLocked === true;

									return (
										<box
											flexDirection="row"
											backgroundColor={
												isCursor() ? theme.bgHighlight : undefined
											}
										>
											<box width={3}>
												<text fg={theme.warning}>
													{isLocked() ? "\u{1F512}" : "  "}
												</text>
											</box>

											<box width={9}>
												<text fg={isLocked() ? theme.warning : theme.sha}>
													{commit.shortSha}
												</text>
											</box>

											<box width={16}>
												<text fg={theme.info}>
													{commit.author.length > 14
														? `${commit.author.substring(0, 14)}..`
														: commit.author.padEnd(14)}
												</text>
											</box>

											<box width={10}>
												<text fg={theme.textDim}>{commit.date.padEnd(8)}</text>
											</box>

											<text
												fg={
													isCursor()
														? theme.cursor
														: isLocked()
															? theme.warning
															: theme.text
												}
												attributes={isCursor() || isLocked() ? 1 : 0}
											>
												{commit.message.length > 55
													? `${commit.message.substring(0, 55)}...`
													: commit.message}
											</text>
										</box>
									);
								}}
							</For>
						</box>
					</scrollbox>
				</Show>
			</Show>

			<box
				flexDirection="row"
				backgroundColor={theme.bgDark}
				paddingLeft={1}
				paddingRight={1}
				flexShrink={0}
				height={1}
			>
				<For each={shortcuts.changelog}>
					{(item) => (
						<>
							<text fg={theme.key}>{item.key}</text>
							<text fg={theme.textDim}> {item.description} </text>
						</>
					)}
				</For>

				<box flexGrow={1} />

				<text fg={theme.success}>+{lockedIndex()} new</text>
				<text fg={theme.warning}> {"\u{1F512}"} </text>
				<text fg={theme.textMuted}>
					{commits().length - lockedIndex() - 1} older
				</text>
			</box>

			<ConfirmDialog
				visible={showConfirm}
				inputName={() => props.input.name}
				commit={confirmCommit}
			/>
		</box>
	);
}
