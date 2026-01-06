import { interruptAll } from "./runtime";

let isShuttingDown = false;
let renderer: { destroy(): void } | undefined;

/**
 * Register the renderer so it can be destroyed on shutdown.
 */
export function setRenderer(r: { destroy(): void }): void {
	renderer = r;
}

/**
 * Shutdown the application: destroy the renderer, interrupt all fibers, and exit.
 */
export async function shutdown(code = 0): Promise<void> {
	if (isShuttingDown) return;
	isShuttingDown = true;

	// Destroy the renderer first to restore terminal state
	renderer?.destroy();

	// Give fibers a chance to clean up, but don't wait forever
	const timeout = new Promise<void>((resolve) => setTimeout(resolve, 1000));
	await Promise.race([interruptAll(), timeout]);

	process.exit(code);
}
