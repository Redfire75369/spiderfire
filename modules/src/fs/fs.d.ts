declare module "assert" {
	export function readBinary(path: string): Promise<Uint8Array>;
	export function readString(path: string): Promise<string>;
	export function readDir(path: string): Promise<string[]>;
	export function write(path: string, contents: string): Promise<boolean>;
	export function createDir(path: string): Promise<boolean>;
	export function createDirRecursive(path: string): Promise<boolean>;
	export function removeFile(path: string): Promise<boolean>;
	export function removeDir(path: string): Promise<boolean>;
	export function removeDirRecursive(path: string): Promise<boolean>;
	export function copy(from: string, to: string): Promise<boolean>;
	export function rename(from: string, to: string): Promise<boolean>;
	export function softLink(original: string, link: string): Promise<boolean>;
	export function hardLink(original: string, link: string): Promise<boolean>;

	export const sync: {
		readBinary(path: string): Uint8Array,
		readString(path: string): string,
		readDir(path: string): string[],
		write(path: string, contents: string): boolean,
		createDir(path: string): boolean,
		createDirRecursive(path: string): boolean,
		removeFile(path: string): boolean,
		removeDir(path: string): boolean,
		removeDirRecursive(path: string): boolean,
		copy(from: string, to: string): boolean,
		rename(from: string, to: string): boolean,
		softLink(original: string, link: string): boolean,
		hardLink(original: string, link: string): boolean,
	};

	namespace Assert {
		function readBinary(path: string): Promise<Uint8Array>;
		function readString(path: string): Promise<string>;
		function readDir(path: string): Promise<string[]>;
		function write(path: string, contents: string): Promise<boolean>;
		function createDir(path: string): Promise<boolean>;
		function createDirRecursive(path: string): Promise<boolean>;
		function removeFile(path: string): Promise<boolean>;
		function removeDir(path: string): Promise<boolean>;
		function removeDirRecursive(path: string): Promise<boolean>;
		function copy(from: string, to: string): Promise<boolean>;
		function rename(from: string, to: string): Promise<boolean>;
		function softLink(original: string, link: string): Promise<boolean>;
		function hardLink(original: string, link: string): Promise<boolean>;
		const sync: {
			readBinary(path: string): Uint8Array,
			readString(path: string): string,
			readDir(path: string): string[],
			write(path: string, contents: string): boolean,
			createDir(path: string): boolean,
			createDirRecursive(path: string): boolean,
			removeFile(path: string): boolean,
			removeDir(path: string): boolean,
			removeDirRecursive(path: string): boolean,
			copy(from: string, to: string): boolean,
			rename(from: string, to: string): boolean,
			softLink(original: string, link: string): boolean,
			hardLink(original: string, link: string): boolean,
		};
	}

	export default Assert;
}
