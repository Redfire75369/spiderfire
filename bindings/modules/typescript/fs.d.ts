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

	import {
		open as openSync,
		create as createSync,

		readDir as readDirSync,
		createDir as createDirSync,
		remove as removeSync,
		copy as copySync,
		rename as renameSync,
		symlink as symlinkSync,
		link as linkSync
	} from "fs/sync";

	export {
		openSync,
		createSync,

		readDirSync,
		createDirSync,
		removeSync,
		copySync,
		renameSync,
		symlinkSync,
		linkSync,
	};

	export const sync: {
		open: typeof openSync,
		create: typeof createSync,

		readDir: typeof readDirSync,
		createDir: typeof createDirSync,
		remove: typeof removeSync,
		copy: typeof copySync,
		rename: typeof renameSync,
		symlink: typeof symlinkSync,
		link: typeof linkSync,
	};

	namespace FileSystem {
		export {
			FileHandle,

			type OpenOptions,
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
