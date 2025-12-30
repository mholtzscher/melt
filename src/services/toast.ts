import type { CliRenderer } from "@opentui/core";
import { toast as baseToast, ToasterRenderable } from "@opentui-ui/toast";
import { theme } from "../theme";

let toaster: ToasterRenderable | null = null;

export function mountToaster(renderer: CliRenderer): void {
	if (toaster) return;

	toaster = new ToasterRenderable(renderer, {
		offset: { bottom: 4 },
		toastOptions: {
			style: {
				backgroundColor: theme.bg,
				foregroundColor: theme.text,
				borderColor: theme.border,
				borderStyle: "rounded",
			},
			success: {
				style: { borderColor: theme.success },
			},
			error: {
				style: { borderColor: theme.error },
			},
			warning: {
				style: { borderColor: theme.warning },
			},
			info: {
				style: { borderColor: theme.info },
			},
			loading: {
				style: { borderColor: theme.accent },
			},
		},
	});

	renderer.root.add(toaster);
}

export const toast = {
	loading(message: string, id?: string | number): string | number | undefined {
		return baseToast.loading(message, id !== undefined ? { id } : undefined);
	},
	success(message: string, id?: string | number): void {
		baseToast.success(message, id !== undefined ? { id } : undefined);
	},
	error(message: string, id?: string | number): void {
		baseToast.error(message, id !== undefined ? { id } : undefined);
	},
	warning(message: string, id?: string | number): void {
		baseToast.warning(message, id !== undefined ? { id } : undefined);
	},
};
