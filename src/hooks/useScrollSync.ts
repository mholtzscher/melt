import { createEffect, on, type Accessor } from "solid-js";

interface ScrollBoxLike {
	height?: number;
	scrollTop?: number;
}

export function useScrollSync(
	cursorIndex: Accessor<number>,
	getScrollBox: () => ScrollBoxLike | undefined,
): void {
	createEffect(
		on(cursorIndex, (cursor) => {
			const scrollBox = getScrollBox();
			if (!scrollBox) return;

			const viewportHeight = scrollBox.height ?? 10;
			const scrollTop = scrollBox.scrollTop ?? 0;

			if (cursor >= scrollTop + viewportHeight) {
				scrollBox.scrollTop = cursor - viewportHeight + 1;
			} else if (cursor < scrollTop) {
				scrollBox.scrollTop = cursor;
			}
		}),
	);
}
