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

	decode(buffer: BufferSource, options?: DecodeOptions): string;
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
