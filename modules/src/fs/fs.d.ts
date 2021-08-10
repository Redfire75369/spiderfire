declare module "assert" {
	export function readBinary(path: string): Uint8Array;
	export function readString(path: string): string;
	export function readDir(path: string): string[];
	export function write(path: string, contents: string): boolean;
	export function createDir(path: string): boolean;
	export function createDirRecursive(path: string): boolean;
	export function removeFile(path: string): boolean;
	export function removeDir(path: string): boolean;
	export function removeDirRecursive(path: string): boolean;
	export function copy(from: string, to: string): boolean;
	export function rename(from: string, to: string): boolean;
	export function softLink(original: string, link: string): boolean;
	export function hardLink(original: string, link: string): boolean;

	namespace Assert {
		 function readBinary(path: string): Uint8Array;
		 function readString(path: string): string;
		 function readDir(path: string): string[];
		 function write(path: string, contents: string): boolean;
		 function createDir(path: string): boolean;
		 function createDirRecursive(path: string): boolean;
		 function removeFile(path: string): boolean;
		 function removeDir(path: string): boolean;
		 function removeDirRecursive(path: string): boolean;
		 function copy(from: string, to: string): boolean;
		 function rename(from: string, to: string): boolean;
		 function softLink(original: string, link: string): boolean;
		 function hardLink(original: string, link: string): boolean;
	}

	export default Assert;
}
