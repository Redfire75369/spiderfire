declare class AbortController {
	constructor();

	get signal(): AbortSignal;

	abort(reason?: any): void;
}

declare class AbortSignal {
	get aborted(): boolean;

	get reason(): any;

	static abort(reason?: any): AbortSignal;

	static timeout(time: number): AbortSignal;

	throwIfAborted(): void;
}
