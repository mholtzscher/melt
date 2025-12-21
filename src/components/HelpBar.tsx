import type { Accessor } from "solid-js";
import { For, Show } from "solid-js";
import type { HelpItem } from "../config/shortcuts";
import { mocha, theme } from "../theme";

export interface HelpBarProps {
	statusMessage: Accessor<string | undefined>;
	loading: Accessor<boolean>;
	selectedCount: Accessor<number>;
	shortcuts: HelpItem[];
}

export function HelpBar(props: HelpBarProps) {
	return (
		<box
			flexDirection="row"
			backgroundColor={theme.bgDark}
			paddingLeft={1}
			paddingRight={1}
		>
			<Show
				when={props.statusMessage()}
				fallback={
					<box flexDirection="row" flexGrow={1}>
						<For each={props.shortcuts}>
							{(item) => (
								<>
									<text fg={mocha.lavender}>{item.key}</text>
									<text fg={theme.textDim}> {item.description} </text>
								</>
							)}
						</For>
					</box>
				}
			>
				<text fg={props.loading() ? theme.warning : theme.info}>
					{props.statusMessage()}
				</text>
			</Show>

			<Show when={props.selectedCount() > 0}>
				<box marginLeft={2}>
					<text fg={theme.selected}>{props.selectedCount()} selected</text>
				</box>
			</Show>
		</box>
	);
}
