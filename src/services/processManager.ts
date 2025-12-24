const controller = new AbortController();

export const processManager = {
	getSignal(): AbortSignal {
		return controller.signal;
	},

	cleanup(): void {
		if (controller.signal.aborted) return;
		controller.abort();
	},
};
