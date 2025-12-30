import type { JSX } from "solid-js";
import { For, Show } from "solid-js";
import type { HelpItem } from "../config/shortcuts";
import { theme } from "../theme";

interface ShortcutItemProps {
	key: string;
	description: string;
}

function ShortcutItem(props: ShortcutItemProps) {
	return (
		<box flexDirection="row" marginRight={1}>
			<text fg={theme.key}>{props.key}</text>
			<text fg={theme.textDim}> {props.description} </text>
		</box>
	);
}

export interface HelpBarProps {
	shortcuts: readonly HelpItem[];
	children?: JSX.Element;
}

export function HelpBar(props: HelpBarProps) {
	return (
		<box
			flexDirection="row"
			justifyContent="space-between"
			paddingLeft={1}
			paddingRight={1}
			flexShrink={0}
			borderStyle="rounded"
			borderColor={theme.border}
		>
			<Show when={props.shortcuts.length > 0}>
				<box flexDirection="row">
					<For each={props.shortcuts}>{(item) => <ShortcutItem key={item.key} description={item.description} />}</For>
				</box>
			</Show>

			{props.children}
		</box>
	);
}
