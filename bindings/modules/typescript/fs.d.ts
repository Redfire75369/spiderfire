declare module "fs" {
	export interface Metadata {
		size: number;

		isFile: boolean;
		isDirectory: boolean;
		isSymlink: boolean;

		created: Date | null;
		accessed: Date | null;
		modified: Date | null;

		readonly: boolean;
	}

	export class FileHandle {
		read(): Promise<Uint8Array>;
		read(array: Uint8Array): Promise<number>;

		readSync(): Uint8Array;
		readSync(array: Uint8Array): number;

		write(source: BufferSource): Promise<number>;
		writeSync(source: BufferSource): number;

		writeAll(source: BufferSource): Promise<void>;
		writeAllSync(source: BufferSource): void;

		truncate(length?: number): Promise<void>;
		truncateSync(length?: number): void;

		sync(): Promise<void>;
		syncSync(): void;

		syncData(): Promise<void>;
		syncDataSync(): void;

		metadata(): Promise<Metadata>;

		metadataSync(): Metadata;
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

	export function metadata(path: string): Promise<Metadata>;

	export function linkMetadata(path: string): Promise<Metadata>;

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

		metadata as metadataSync,
		linkMetadata as linkMetadataSync,

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

		metadataSync,
		linkMetadataSync,

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

		metadata: typeof metadataSync,
		linkMetadata: typeof linkMetadataSync,

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
			type Metadata,
			FileHandle,

			type OpenOptions,
			open,
			create,

			metadata,
			linkMetadata,

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
	import {type FileHandle, type Metadata, type OpenOptions} from "fs";

	export function open(path: string, options?: OpenOptions): FileHandle;

	export function create(path: string): FileHandle;

	export function metadata(path: string): Metadata;

	export function linkMetadata(path: string): Metadata;

	export function readDir(path: string): string[];

	export function createDir(path: string, recursive?: boolean): void;

	export function remove(path: string, recursive?: boolean): void;

	export function copy(from: string, to: string): number;

	export function rename(from: string, to: string): void;

	export function symlink(original: string, link: string): void;

	export function link(original: string, link: string): void;
}
