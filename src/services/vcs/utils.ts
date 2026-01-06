import type { FlakeInput } from "../../types";
import type { ForgeType, RepoInfo } from "./types";

/** Known forge hosts and their types */
const FORGE_HOSTS: Record<string, ForgeType> = {
	"github.com": "github",
	"gitlab.com": "gitlab",
	"codeberg.org": "codeberg",
	"git.sr.ht": "sourcehut",
	// Common self-hosted GitLab instances
	"gitlab.gnome.org": "gitlab",
	"gitlab.freedesktop.org": "gitlab",
	"gitlab.alpinelinux.org": "gitlab",
	"invent.kde.org": "gitlab",
	// Common Gitea/Forgejo instances
	"gitea.com": "gitea",
	"forgejo.org": "gitea",
	"notabug.org": "gitea",
};

/**
 * Format a date string as short relative time (e.g., "2d ago")
 */
export function formatShortRelativeTime(dateStr: string): string {
	const date = new Date(dateStr);
	const now = new Date();
	const diffMs = now.getTime() - date.getTime();
	const diffSecs = Math.floor(diffMs / 1000);

	if (diffSecs < 60) {
		return "just now";
	}

	const TIME_UNITS = [
		{ seconds: 365 * 24 * 60 * 60, short: "y" },
		{ seconds: 30 * 24 * 60 * 60, short: "mo" },
		{ seconds: 7 * 24 * 60 * 60, short: "w" },
		{ seconds: 24 * 60 * 60, short: "d" },
		{ seconds: 60 * 60, short: "h" },
		{ seconds: 60, short: "m" },
	];

	for (const unit of TIME_UNITS) {
		if (diffSecs >= unit.seconds) {
			const count = Math.floor(diffSecs / unit.seconds);
			// For months and beyond, use date format
			if (unit.short === "mo" || unit.short === "y") {
				return date.toLocaleDateString("en-US", {
					month: "short",
					day: "numeric",
				});
			}
			return `${count}${unit.short} ago`;
		}
	}

	return "just now";
}

/**
 * Format a Unix timestamp as short relative time
 */
export function formatTimestamp(timestamp: number): string {
	return formatShortRelativeTime(new Date(timestamp * 1000).toISOString());
}

/**
 * Parse a git URL to extract host and path components
 * Handles various URL formats:
 * - https://github.com/owner/repo
 * - https://github.com/owner/repo.git
 * - git@github.com:owner/repo.git
 * - git+ssh://git@github.com/owner/repo
 * - ssh://git@github.com/owner/repo.git
 */
export function parseGitUrl(url: string): { host: string; path: string } | null {
	// Remove common prefixes
	const normalized = url
		.replace(/^git\+/, "")
		.replace(/^ssh:\/\//, "")
		.replace(/\.git$/, "");

	// Handle SCP-style URLs (git@host:path)
	const scpMatch = normalized.match(/^git@([^:]+):(.+)$/);
	if (scpMatch?.[1] && scpMatch[2]) {
		return { host: scpMatch[1], path: scpMatch[2] };
	}

	// Handle standard URLs
	try {
		const parsed = new URL(normalized.startsWith("http") ? normalized : `https://${normalized}`);
		const path = parsed.pathname.replace(/^\//, "");
		return { host: parsed.host, path };
	} catch {
		return null;
	}
}

/**
 * Extract owner and repo from a path like "owner/repo" or "owner/repo/..."
 */
export function extractOwnerRepo(path: string): { owner: string; repo: string } | null {
	const parts = path.split("/").filter(Boolean);
	if (parts.length < 2) return null;

	const first = parts[0];
	const second = parts[1];
	if (!first || !second) return null;

	// Handle sourcehut paths like "~owner/repo"
	const owner = first.startsWith("~") ? first.substring(1) : first;

	return { owner, repo: second };
}

/**
 * Detect forge type from a URL (handles both git URLs and nix flake URLs)
 */
export function detectForgeFromUrl(url: string): ForgeType {
	// Check for nix flake URL schemes first
	if (url.startsWith("github:")) return "github";
	if (url.startsWith("gitlab:")) return "gitlab";
	if (url.startsWith("sourcehut:")) return "sourcehut";

	const parsed = parseGitUrl(url);
	if (!parsed) return "generic";

	// Check known hosts
	const forgeType = FORGE_HOSTS[parsed.host];
	if (forgeType) return forgeType;

	// Check for common patterns in host name
	if (parsed.host.includes("gitlab")) return "gitlab";
	if (parsed.host.includes("gitea")) return "gitea";
	if (parsed.host.includes("forgejo")) return "gitea";
	if (parsed.host.includes("gogs")) return "gitea"; // Gogs has similar API

	return "generic";
}

/**
 * Parse repository info from a FlakeInput
 * Uses explicit owner/repo if available, otherwise parses from URL
 */
export function parseRepoInfo(input: FlakeInput): RepoInfo | null {
	// Skip non-git inputs
	if (input.type !== "git") {
		return null;
	}

	// If owner/repo are available, detect forge from URL
	if (input.owner && input.repo) {
		const forge = detectForgeFromUrl(input.url);
		const parsed = parseGitUrl(input.url);

		// Normalize sourcehut owner (strip ~ prefix, it gets added back in git provider)
		const owner = input.owner.startsWith("~") ? input.owner.substring(1) : input.owner;

		return {
			forge,
			owner,
			repo: input.repo,
			host: parsed?.host,
		};
	}

	// Parse owner/repo from URL
	if (input.url) {
		const parsed = parseGitUrl(input.url);
		if (!parsed) return null;

		const ownerRepo = extractOwnerRepo(parsed.path);
		if (!ownerRepo) return null;

		const forge = detectForgeFromUrl(input.url);

		return {
			forge,
			owner: ownerRepo.owner,
			repo: ownerRepo.repo,
			host: parsed.host,
		};
	}

	return null;
}

/**
 * Check if a revision matches (handles full SHA and short SHA)
 */
export function revisionMatches(sha: string, targetRev: string): boolean {
	return sha === targetRev || sha.startsWith(targetRev) || targetRev.startsWith(sha);
}

/**
 * Get cache directory following XDG spec
 */
export function getCacheDir(): string {
	const xdgCache = process.env.XDG_CACHE_HOME;
	if (xdgCache) {
		return `${xdgCache}/melt`;
	}
	const home = process.env.HOME || "~";
	return `${home}/.cache/melt`;
}
