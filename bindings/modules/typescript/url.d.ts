declare module "url" {
	export function domainToASCII(domain: string, strict?: boolean): string;

	export function domainToUnicode(domain: string): string;

	namespace Url {
		export {
			domainToASCII,
			domainToUnicode,
		};
	}

	export default Url;
}
