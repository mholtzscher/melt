import { Effect, Fiber, MutableRef } from "effect";

// Track all running fibers for cleanup on process exit
const runningFibers = MutableRef.make<Set<Fiber.RuntimeFiber<unknown, unknown>>>(new Set());

/**
 * Run an Effect and track it for cleanup on process exit
 */
export function runEffect<A, E>(effect: Effect.Effect<A, E>): Promise<A> {
	return new Promise((resolve, reject) => {
		const fiber = Effect.runFork(effect);

		// Track the fiber
		MutableRef.update(runningFibers, (set) => {
			set.add(fiber);
			return set;
		});

		// Wait for completion and cleanup
		Effect.runPromise(Fiber.join(fiber))
			.then((result) => {
				MutableRef.update(runningFibers, (set) => {
					set.delete(fiber);
					return set;
				});
				resolve(result);
			})
			.catch((error) => {
				MutableRef.update(runningFibers, (set) => {
					set.delete(fiber);
					return set;
				});
				reject(error);
			});
	});
}

/**
 * Run an Effect with Either for error handling, tracked for cleanup
 */
export function runEffectEither<A, E>(
	effect: Effect.Effect<A, E>,
): Promise<{ _tag: "Right"; right: A } | { _tag: "Left"; left: E }> {
	return runEffect(Effect.either(effect)) as Promise<{ _tag: "Right"; right: A } | { _tag: "Left"; left: E }>;
}

/**
 * Interrupt all running fibers - call this on SIGINT/SIGTERM
 */
export async function interruptAll(): Promise<void> {
	const fibers = MutableRef.get(runningFibers);
	if (fibers.size === 0) return;

	const interrupts = Array.from(fibers).map((fiber) =>
		Effect.runPromise(Fiber.interrupt(fiber)).catch(() => {
			// Ignore errors during interruption
		}),
	);

	await Promise.all(interrupts);
	MutableRef.set(runningFibers, new Set());
}
