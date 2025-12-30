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
	loading(message: string, id?: string | number): string | number {
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

export interface ToastMeta {
	id: string;
	message: string;
}

export function toastForError(errorMsg: string): ToastMeta {
	const normalized = errorMsg.toLowerCase();

	const patterns: Array<[RegExp, ToastMeta]> = [
		[/rate limit/, { id: "error:rate-limit", message: "GitHub rate limit exceeded; set GITHUB_TOKEN" }],
		[/bad credentials|requires authentication/, { id: "error:auth", message: "GitHub authentication failed - check GITHUB_TOKEN" }],
		[/404|not found/, { id: "error:not-found", message: "GitHub repository not found" }],
		[/fetch failed|enotfound|network/, { id: "error:network", message: "Network error checking GitHub" }],
		[/missing owner or repo/, { id: "error:missing-owner-repo", message: "Invalid GitHub input (missing owner/repo)" }],
		[/github api error/, { id: "error:github-api", message: "GitHub API error checking updates" }],
	];

	const match = patterns.find(([regex]) => regex.test(normalized));
	return match?.[1] ?? { id: "error:unknown", message: "Error checking updates" };
}
