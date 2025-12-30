import type { Accessor } from "solid-js";
import { Show } from "solid-js";
import { theme } from "../theme";
import type { GitHubCommit } from "../types";

export interface ConfirmDialogProps {
	visible: Accessor<boolean>;
	inputName: Accessor<string>;
	commit: Accessor<GitHubCommit | undefined>;
}

export function ConfirmDialog(props: ConfirmDialogProps) {
	const commitMessage = () => {
		const msg = props.commit()?.message;
		return msg && msg.length > 40 ? `${msg.substring(0, 40)}...` : (msg ?? "");
	};

	return (
		<Show when={props.visible() && props.commit()}>
			<box position="absolute" top={0} left={0} right={0} bottom={0} alignItems="center" justifyContent="center">
				<box
					flexDirection="column"
					backgroundColor={theme.bgDark}
					borderStyle="rounded"
					borderColor={theme.accent}
					padding={1}
					minWidth={45}
				>
					<text fg={theme.text} attributes={1}>
						Lock {props.inputName()} to {props.commit()?.shortSha}?
					</text>

					<box height={1} marginTop={1}>
						<text fg={theme.textDim}>{commitMessage()}</text>
					</box>

					<box marginTop={1} flexDirection="row" justifyContent="center">
						<text fg={theme.success}>y</text>
						<text fg={theme.textDim}> confirm </text>
						<text fg={theme.error}>n/q</text>
						<text fg={theme.textDim}> cancel</text>
					</box>
				</box>
			</box>
		</Show>
	);
}
