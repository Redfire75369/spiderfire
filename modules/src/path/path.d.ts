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
		function join(...segments: string[]): string;
		function stripPrefix(path: string, prefix: string): string;
		function fileStem(path: string): string | null;
		function parent(path: string): string | null;
		function fileName(path: string): string | null;
		function extension(path: string): string | null;
		function withFileName(path: string, fileName: string): string;
		function withExtension(path: string, extension: string): string;
		function isAbsolute(path: string): boolean;
		function isRelative(path: string): boolean;
		function hasRoot(path: string): boolean;
		function startsWith(path: string, prefix: string): boolean;
		function endsWith(path: string, suffix: string): boolean;

		const separator: string;
		const delimiter: string;
	}

	export default Path;
}

