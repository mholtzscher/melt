import { dirname, resolve } from "node:path";
import { $ } from "bun";
import type {
	FlakeInput,
	FlakeInputType,
	FlakeMetadata,
	NixFlakeMetadataResponse,
} from "./types";

/**
 * Resolve a path argument to a flake directory
 */
export function resolveFlakePath(path: string): string {
	const resolved = resolve(path);
	if (resolved.endsWith("flake.nix")) {
		return dirname(resolved);
	}
	return resolved;
}

/**
 * Check if a flake.nix exists in the given path
 */
export async function hasFlakeNix(path: string = "."): Promise<boolean> {
	const file = Bun.file(`${path}/flake.nix`);
	return file.exists();
}

/**
 * Determine the input type from the locked/original data
 */
function getInputType(
	locked?: { type: string },
	original?: { type: string },
): FlakeInputType {
	const type = locked?.type || original?.type || "other";

	switch (type) {
		case "github":
			return "github";
		case "gitlab":
			return "gitlab";
		case "sourcehut":
			return "sourcehut";
		case "path":
			return "path";
		case "git":
			return "git";
		default:
			return "other";
	}
}

/**
 * Get the URL string for an input
 */
function getInputUrl(
	locked?: {
		type: string;
		owner?: string;
		repo?: string;
		url?: string;
		path?: string;
	},
	original?: {
		type: string;
		owner?: string;
		repo?: string;
		url?: string;
		path?: string;
	},
): string {
	const data = original || locked;
	if (!data) return "unknown";

	switch (data.type) {
		case "github":
			return `github:${data.owner}/${data.repo}`;
		case "gitlab":
			return `gitlab:${data.owner}/${data.repo}`;
		case "sourcehut":
			return `sourcehut:${data.owner}/${data.repo}`;
		case "path":
			return data.path || "path:unknown";
		case "git":
			return data.url || "git:unknown";
		default:
			return data.url || "unknown";
	}
}

/**
 * Get flake metadata from the current directory
 */
export async function getFlakeMetadata(
	path: string = ".",
): Promise<FlakeMetadata> {
	const result = await $`nix flake metadata --json ${path} 2>/dev/null`.text();
	const data: NixFlakeMetadataResponse = JSON.parse(result);

	const rootNode = data.locks.nodes[data.locks.root];
	if (!rootNode || !rootNode.inputs) {
		return {
			description: data.description,
			inputs: [],
			path,
		};
	}

	const directInputNames = Object.keys(rootNode.inputs);

	const inputs: FlakeInput[] = [];

	for (const name of directInputNames) {
		const inputRef = rootNode.inputs[name];
		const nodeName = Array.isArray(inputRef) ? inputRef[0] : inputRef;

		if (typeof nodeName !== "string") continue;

		const node = data.locks.nodes[nodeName];
		if (!node?.locked) continue;

		const locked = node.locked;
		const original = node.original;

		inputs.push({
			name,
			type: getInputType(locked, original),
			owner: locked.owner || original?.owner,
			repo: locked.repo || original?.repo,
			ref: original?.ref, // branch/tag reference (e.g., "nixos-unstable")
			url: getInputUrl(locked, original),
			rev: locked.rev || "",
			shortRev: locked.rev?.substring(0, 7) || "",
			lastModified: locked.lastModified || 0,
		});
	}

	return {
		description: data.description,
		inputs,
		path,
	};
}

/**
 * Update specific flake inputs
 */
export async function updateInputs(
	inputNames: string[],
	path: string = ".",
): Promise<{ success: boolean; output: string }> {
	try {
		const args = inputNames.join(" ");
		const result =
			await $`nix flake update ${args} --flake ${path} 2>&1`.text();
		return { success: true, output: result };
	} catch (error) {
		return {
			success: false,
			output: error instanceof Error ? error.message : String(error),
		};
	}
}

/**
 * Update all flake inputs
 */
export async function updateAll(
	path: string = ".",
): Promise<{ success: boolean; output: string }> {
	try {
		const result = await $`nix flake update --flake ${path} 2>&1`.text();
		return { success: true, output: result };
	} catch (error) {
		return {
			success: false,
			output: error instanceof Error ? error.message : String(error),
		};
	}
}

/**
 * Lock a specific input to a specific revision
 */
export async function lockInputToRev(
	inputName: string,
	rev: string,
	owner: string,
	repo: string,
	path: string = ".",
): Promise<{ success: boolean; output: string }> {
	try {
		const overrideUrl = `github:${owner}/${repo}/${rev}`;
		const result =
			await $`nix flake update ${inputName} --override-input ${inputName} ${overrideUrl} --flake ${path} 2>&1`.text();
		return { success: true, output: result };
	} catch (error) {
		return {
			success: false,
			output: error instanceof Error ? error.message : String(error),
		};
	}
}

// Re-export formatRelativeTime from shared time utilities
export { formatRelativeTime } from "./time";
