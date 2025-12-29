import type { CliRenderer } from "@opentui/core";
import { TOAST_DURATION, ToasterRenderable, toast as baseToast } from "@opentui-ui/toast";
import { theme } from "../theme";

type ToastId = string | number;
type LoadingOptions = NonNullable<Parameters<typeof baseToast.loading>[1]>;

let initialized = false;
const pendingQueue: Array<() => void> = [];

function generateToastId(): ToastId {
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
	if (initialized) return;

	const t = new ToasterRenderable(renderer, {
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

	renderer.root.add(t);
	initialized = true;
	flushQueue();
}

export const toast = {
	loading(
		message: Parameters<typeof baseToast.loading>[0],
		opts?: Parameters<typeof baseToast.loading>[1],
	): ReturnType<typeof baseToast.loading> {
		if (!initialized) {
			const id = (opts as { id?: ToastId } | undefined)?.id ?? generateToastId();
			const optsWithId: LoadingOptions = { ...((opts ?? {}) as LoadingOptions), id };
			pendingQueue.push(() => baseToast.loading(message, optsWithId));
			return id;
		}
		return baseToast.loading(message, opts);
	},
	success(
		message: Parameters<typeof baseToast.success>[0],
		opts?: Parameters<typeof baseToast.success>[1],
	): void {
		if (!initialized) {
			pendingQueue.push(() => baseToast.success(message, opts));
			return;
		}
		baseToast.success(message, opts);
	},
	error(
		message: Parameters<typeof baseToast.error>[0],
		opts?: Parameters<typeof baseToast.error>[1],
	): void {
		if (!initialized) {
			pendingQueue.push(() => baseToast.error(message, opts));
			return;
		}
		baseToast.error(message, opts);
	},
	warning(
		message: Parameters<typeof baseToast.warning>[0],
		opts?: Parameters<typeof baseToast.warning>[1],
	): void {
		if (!initialized) {
			pendingQueue.push(() => baseToast.warning(message, opts));
			return;
		}
		baseToast.warning(message, opts);
	},
	info(
		message: Parameters<typeof baseToast.info>[0],
		opts?: Parameters<typeof baseToast.info>[1],
	): void {
		if (!initialized) {
			pendingQueue.push(() => baseToast.info(message, opts));
			return;
		}
		baseToast.info(message, opts);
	},
	dismiss(id?: Parameters<typeof baseToast.dismiss>[0]): void {
		if (!initialized) {
			pendingQueue.push(() => baseToast.dismiss(id));
			return;
		}
		baseToast.dismiss(id);
	},
};
