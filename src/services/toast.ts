import type { CliRenderer } from "@opentui/core";
import { toast, ToasterRenderable, TOAST_DURATION } from "@opentui-ui/toast";
import { theme } from "../theme";

let toasterInitialized = false;

export function initToaster(renderer: CliRenderer): void {
	if (toasterInitialized) return;

	const toaster = new ToasterRenderable(renderer, {
		position: "top-right",
		gap: 1,
		stackingMode: "stack",
		visibleToasts: 3,
		maxWidth: 50,
		offset: {
			top: 1,
			right: 2,
			bottom: 1,
			left: 2,
		},
		toastOptions: {
			style: {
				backgroundColor: theme.bg,
				foregroundColor: theme.text,
				borderColor: theme.border,
				borderStyle: "rounded",
				paddingX: 1,
				paddingY: 0,
			},
			duration: TOAST_DURATION.DEFAULT,
			success: {
				style: { borderColor: theme.success },
				duration: TOAST_DURATION.SHORT,
			},
			error: {
				style: { borderColor: theme.error },
				duration: TOAST_DURATION.LONG,
			},
			warning: {
				style: { borderColor: theme.warning },
				duration: TOAST_DURATION.LONG,
			},
			info: {
				style: { borderColor: theme.info },
				duration: TOAST_DURATION.DEFAULT,
			},
			loading: {
				style: { borderColor: theme.accent },
			},
		},
	});

	renderer.root.add(toaster);
	toasterInitialized = true;
}

export { toast };
