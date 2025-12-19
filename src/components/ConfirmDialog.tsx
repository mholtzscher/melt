import type { Accessor } from "solid-js";
import { Show } from "solid-js";
import { mocha, theme } from "../theme";
import type { GitHubCommit } from "../types";

export interface ConfirmDialogProps {
	visible: Accessor<boolean>;
	inputName: Accessor<string>;
	commit: Accessor<GitHubCommit | undefined>;
}

export function ConfirmDialog(props: ConfirmDialogProps) {
	const commitMessage = () => {
		const c = props.commit();
		if (!c?.message) return "";
		return c.message.length > 40
			? `${c.message.substring(0, 40)}...`
			: c.message;
	};

	return (
		<Show when={props.visible() && props.commit()}>
			<box
				position="absolute"
				top={0}
				left={0}
				right={0}
				bottom={0}
				alignItems="center"
				justifyContent="center"
			>
				<box
					flexDirection="column"
					backgroundColor={theme.bgDark}
					borderStyle="rounded"
					borderColor={theme.accent}
					padding={1}
					minWidth={45}
				>
					<box height={1} flexDirection="row">
						<text fg={theme.text} attributes={1}>
							Lock {props.inputName()} to {props.commit()?.shortSha}?
						</text>
					</box>

					<box height={1} marginTop={1}>
						<text fg={theme.textDim}>{commitMessage()}</text>
					</box>

					<box
						height={1}
						marginTop={1}
						flexDirection="row"
						justifyContent="center"
					>
						<text fg={mocha.green}>[y]</text>
						<text fg={theme.textDim}> confirm </text>
						<text fg={mocha.red}>[n/q]</text>
						<text fg={theme.textDim}> cancel</text>
					</box>
				</box>
			</box>
		</Show>
	);
}
