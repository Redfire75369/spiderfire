declare module "fs" {
	export class FileHandle {
		read(): Promise<Uint8Array>;

		readSync(): Uint8Array;

		readString(): Promise<string>;

		readStringSync(): string;

		write(source: BufferSource): Promise<void>;

		writeSync(source: BufferSource): void;
	}

	export interface OpenOptions {
		read?: boolean,
		write?: boolean,
		append?: boolean,
		create?: boolean,
		createNew?: boolean,
	}

	export function open(path: string, options?: OpenOptions): Promise<FileHandle>;

	export function create(path: string): Promise<FileHandle>;

	export function readDir(path: string): Promise<string[]>;

	export function createDir(path: string, recursive?: boolean): Promise<void>;

	export function remove(path: string, recursive?: boolean): Promise<void>;

	export function copy(from: string, to: string): Promise<number>;

	export function rename(from: string, to: string): Promise<void>;

	export function symlink(original: string, link: string): Promise<void>;

	export function link(original: string, link: string): Promise<void>;

	export const sync: {
		open(path: string, options?: OpenOptions): FileHandle;
		create(path: string): FileHandle;

		readDir(path: string): string[],
		createDir(path: string, recursive?: boolean): void,
		remove(path: string, recursive?: boolean): void,
		copy(from: string, to: string): number,
		rename(from: string, to: string): void,
		symlink(original: string, link: string): void,
		link(original: string, link: string): void,
	};

	namespace FileSystem {
		export {
			FileHandle,

			OpenOptions,
			open,
			create,

			readDir,
			createDir,
			remove,
			copy,
			rename,
			symlink,
			link,

			sync,
		};
	}

	export default FileSystem;
}

declare module "fs/sync" {
	import {FileHandle, OpenOptions} from "fs";

	export function open(path: string, options?: OpenOptions): FileHandle;

	export function create(path: string): FileHandle;

	export function readDir(path: string): string[];

	export function createDir(path: string, recursive?: boolean): void;

	export function remove(path: string, recursive?: boolean): void;

	export function copy(from: string, to: string): number;

	export function rename(from: string, to: string): void;

	export function symlink(original: string, link: string): void;

	export function link(original: string, link: string): void;
}
