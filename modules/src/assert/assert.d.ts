declare module "assert" {
	export function assert(assertion?: boolean, message?: string): void;
	export function debugAssert(assertion?: boolean, message?: string): void;

	namespace Assert {
		function assert(assertion?: boolean, message?: string): void;
		function debugAssert(assertion?: boolean, message?: string): void;
	}

	export default Assert;
}
