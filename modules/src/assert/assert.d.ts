declare module "assert" {
	export function assert(assertion?: boolean): void;
	export function debugAssert(assertion?: boolean): void;

	namespace Assert {
		function assert(assertion?: boolean): void;
		function debugAssert(assertion?: boolean): void;
	}

	export default Assert;
}
