import type { CliRenderer } from "@opentui/core";
import { toast as baseToast, TOAST_DURATION, ToasterRenderable } from "@opentui-ui/toast";
import { theme } from "../theme";

let toaster: ToasterRenderable | null = null;

export function mountToaster(renderer: CliRenderer): void {
	if (toaster) return;

	toaster = new ToasterRenderable(renderer, {
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
}

export const toast = {
	loading(
		message: Parameters<typeof baseToast.loading>[0],
		opts?: Parameters<typeof baseToast.loading>[1],
	): string | number | undefined {
		return baseToast.loading(message, opts);
	},
	success(message: Parameters<typeof baseToast.success>[0], opts?: Parameters<typeof baseToast.success>[1]): void {
		baseToast.success(message, opts);
	},
	error(message: Parameters<typeof baseToast.error>[0], opts?: Parameters<typeof baseToast.error>[1]): void {
		baseToast.error(message, opts);
	},
	warning(message: Parameters<typeof baseToast.warning>[0], opts?: Parameters<typeof baseToast.warning>[1]): void {
		baseToast.warning(message, opts);
	},
};
