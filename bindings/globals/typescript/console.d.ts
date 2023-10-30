declare namespace console {
	function log(...values: any[]): void;

	function info(...values: any[]): void;

	function dir(...values: any[]): void;

	function dirxml(...values: any[]): void;

	function warn(...values: any[]): void;

	function error(...values: any[]): void;

	function debug(...values: any[]): void;

	function assert(assertion: boolean, ...values: any[]): void;

	function clear(): void;

	function trace(...values: any[]): void;

	function group(...values: any[]): void;

	function groupCollapsed(...values: any[]): void;

	function groupEnd(): void;

	function count(label?: string): void;

	function countReset(label?: string): void;

	function time(label?: string): void;

	function timeLog(label?: string, ...values: any[]): void;

	function timeEnd(label?: string): void;

	function table(data: any, columns?: string[]): void;
}
