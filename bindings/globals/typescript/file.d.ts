declare type BlobPart = BufferSource | Blob | String;

declare type EndingType = "transparent" | "native";

declare interface BlobOptions {
	type?: string,
	endings?: EndingType,
}

declare class Blob {
	constructor(blobParts?: BlobPart[], options?: BlobOptions);

	get size(): number;

	get type(): string;

	slice(start?: number, end?: number, type?: string): Blob;

	text(): Promise<string>;

	arrayBuffer(): Promise<ArrayBuffer>;
}

declare interface FileOptions extends BlobOptions {
	lastModified?: number,
}

declare class File extends Blob {
	constructor(blobParts?: BlobPart[], options?: FileOptions);

	get name(): string;

	get lastModified(): number;
}

declare class FileReader {
	static EMPTY: number;
	static LOADING: number;
	static DONE: number;

	constructor();

	get readyState(): number;

	get result(): string | ArrayBuffer | null;

	get error(): Error | null;

	readAsArrayBuffer(blob: Blob): void;

	readAsBinaryString(blob: Blob): void;

	readAsText(blob: Blob, encoding?: string): void;

	readAsDataURL(blob: Blob): void;
}


declare class FileReaderSync {
	constructor();

	readAsArrayBuffer(blob: Blob): ArrayBuffer;

	readAsBinaryString(blob: Blob): string;

	readAsText(blob: Blob, encoding?: string): string;

	readAsDataURL(blob: Blob): string;
}
