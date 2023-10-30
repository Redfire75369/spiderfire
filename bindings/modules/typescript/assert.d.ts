declare module "assert" {
	export function ok(assertion?: boolean, message?: string): void;

	export function equals(actual: any, expected: any, message?: string): void;

	export function throws(func: () => void, message?: string): void;

	export function fail(message?: string): void;

	namespace Assert {
		export {
			ok,
			equals,
			throws,
			fail,
		};
	}

	export default Assert;
}
