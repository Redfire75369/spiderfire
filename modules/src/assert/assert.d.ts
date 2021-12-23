declare module "assert" {
	export function assert(assertion?: boolean, message?: string): void;
	export function equals(actual: any, expected: any, message?: string): void;
	export function throws(func: () => void, message?: string): void;
	export function fail(message?: string): void;

	namespace Assert {
		export {
			assert,
			equals,
			throws,
			fail,
		};
	}

	export default Assert;
}
