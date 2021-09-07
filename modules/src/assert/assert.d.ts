declare module "assert" {
	export function assert(assertion?: boolean, message?: string): void;
	export function equal(actual: any, expected: any, message?: string): void;
	export function throws(func: () => void , message?: string): void;
	export function fail(message?: string): void;

	namespace Assert {
		function assert(assertion?: boolean, message?: string): void;
		function equal(actual: any, expected: any, message?: string): void;
		function throws(func: () => void , message?: string): void;
		function fail(message?: string): void;
	}

	export default Assert;
}
