import type { CliRenderer } from "@opentui/core";
import { toast as baseToast, TOAST_DURATION, ToasterRenderable } from "@opentui-ui/toast";
import { theme } from "../theme";

let toaster: ToasterRenderable | null = null;
const pendingQueue: Array<() => void> = [];

function generateToastId(): string | number {
	if (typeof globalThis.crypto?.randomUUID === "function") {
		return globalThis.crypto.randomUUID();
	}
	return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function flushQueue() {
	while (pendingQueue.length > 0) {
		const fn = pendingQueue.shift();
		fn?.();
	}
}

export function initToaster(renderer: CliRenderer): void {
	if (toaster) return;

	toaster = new ToasterRenderable(renderer, {
		position: "top-right",
		gap: 1,
		stackingMode: "stack",
		visibleToasts: 3,
		maxWidth: 50,
		offset: { top: 1, right: 2, bottom: 1, left: 2 },
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
	flushQueue();
}

export const toast = {
	loading(
		message: Parameters<typeof baseToast.loading>[0],
		opts?: Parameters<typeof baseToast.loading>[1],
	): string | number | undefined {
		if (!toaster) {
			const id = opts?.id ?? generateToastId();
			pendingQueue.push(() => baseToast.loading(message, { ...opts, id }));
			return id;
		}
		return baseToast.loading(message, opts);
	},
	success(message: Parameters<typeof baseToast.success>[0], opts?: Parameters<typeof baseToast.success>[1]): void {
		if (!toaster) {
			pendingQueue.push(() => baseToast.success(message, opts));
			return;
		}
		baseToast.success(message, opts);
	},
	error(message: Parameters<typeof baseToast.error>[0], opts?: Parameters<typeof baseToast.error>[1]): void {
		if (!toaster) {
			pendingQueue.push(() => baseToast.error(message, opts));
			return;
		}
		baseToast.error(message, opts);
	},
	warning(message: Parameters<typeof baseToast.warning>[0], opts?: Parameters<typeof baseToast.warning>[1]): void {
		if (!toaster) {
			pendingQueue.push(() => baseToast.warning(message, opts));
			return;
		}
		baseToast.warning(message, opts);
	},
	info(message: Parameters<typeof baseToast.info>[0], opts?: Parameters<typeof baseToast.info>[1]): void {
		if (!toaster) {
			pendingQueue.push(() => baseToast.info(message, opts));
			return;
		}
		baseToast.info(message, opts);
	},
	dismiss(id?: Parameters<typeof baseToast.dismiss>[0]): void {
		if (!toaster) {
			pendingQueue.push(() => baseToast.dismiss(id));
			return;
		}
		baseToast.dismiss(id);
	},
};
