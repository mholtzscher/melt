import { Effect, Exit, FiberSet, Scope } from "effect";

// A global scope for the lifetime of the app. Any fibers added to the FiberSet
// will be interrupted when this scope is closed.
const scope = Effect.runSync(Scope.make());

// FiberSet requires a Scope; extend it into our global scope.
const fiberSet = Effect.runSync(Scope.extend(FiberSet.make(), scope));

// A run function that forks effects into the FiberSet and returns a Promise.
const runPromise = Effect.runSync(FiberSet.runtimePromise(fiberSet)());

let isClosed = false;

/**
 * Check if the runtime has been closed.
 */
export function isRuntimeClosed(): boolean {
	return isClosed;
}

/**
 * Run an Effect and track it for cleanup on shutdown.
 */
export function runEffect<A, E>(effect: Effect.Effect<A, E>): Promise<A> {
	if (isClosed) {
		return Promise.reject(new Error("Runtime is closed"));
	}
	return runPromise(effect);
}

/**
 * Run an Effect with Either for error handling, tracked for cleanup.
 */
export function runEffectEither<A, E>(
	effect: Effect.Effect<A, E>,
): Promise<{ _tag: "Right"; right: A } | { _tag: "Left"; left: E }> {
	return runEffect(Effect.either(effect)) as Promise<{ _tag: "Right"; right: A } | { _tag: "Left"; left: E }>;
}

/**
 * Interrupt all running fibers and close the global scope.
 */
export async function interruptAll(): Promise<void> {
	if (isClosed) return;
	isClosed = true;
	await Effect.runPromise(FiberSet.clear(fiberSet).pipe(Effect.andThen(Scope.close(scope, Exit.void))));
}
