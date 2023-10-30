declare module "path" {
	export function join(...segments: string[]): string;

	export function stripPrefix(path: string, prefix: string): string;

	export function fileStem(path: string): string | null;

	export function parent(path: string): string | null;

	export function fileName(path: string): string | null;

	export function extension(path: string): string | null;

	export function withFileName(path: string, fileName: string): string;

	export function withExtension(path: string, extension: string): string;

	export function isAbsolute(path: string): boolean;

	export function isRelative(path: string): boolean;

	export function hasRoot(path: string): boolean;

	export function startsWith(path: string, prefix: string): boolean;

	export function endsWith(path: string, suffix: string): boolean;

	export const separator: string;
	export const delimiter: string;

	namespace Path {
		export {
			join,
			stripPrefix,
			fileStem,
			parent,
			fileName,
			extension,
			withFileName,
			withExtension,
			isAbsolute,
			isRelative,
			hasRoot,
			startsWith,
			endsWith,

			separator,
			delimiter,
		};
	}

	export default Path;
}

