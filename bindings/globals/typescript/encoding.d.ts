declare type TypedArray = Int8Array | Int16Array | Int32Array
	| Uint8Array | Uint8ClampedArray | Uint16Array | Uint32Array
	| Float32Array | Float64Array;

declare interface DecoderOptions {
	fatal: boolean;
	ignoreBOM: boolean;
}
declare interface DecodeOptions {
	stream: boolean;
}

declare class TextDecoder {
	constructor(label?: string, options?: DecoderOptions);

	get encoding(): string;
	get fatal(): boolean;
	get ignoreBOM(): boolean;

	decode(buffer: ArrayBuffer | TypedArray | DataView, options?: DecodeOptions): string;
}

declare interface EncodeResult {
	read: number;
	written: number;
}

declare class TextEncoder {
	get encoding(): string;

	encode(input?: string): Uint8Array;
	encodeInto(source: string, destination: Uint8Array): EncodeResult;
}
