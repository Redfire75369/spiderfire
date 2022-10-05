declare module "http" {
	export type Header = string | string[];
	export type HeaderEntries = [string, string][];
	export interface HeadersObject {
		[key: string]: Header,
	}
	export type HeadersInit = Headers | HeaderEntries | HeadersObject;

	type TypedArray = Int8Array | Int16Array | Int32Array
		| Uint8Array | Uint8ClampedArray | Uint16Array | Uint32Array
		| Float32Array | Float64Array;
	export type Body = string | String | ArrayBuffer | TypedArray | DataView;

	interface CommonOptions {
		setHost?: boolean,
		headers?: HeadersInit,
		body?: Body,
	}

	export type RequestOptions = CommonOptions & {
		auth?: string,
	};

	export type RequestBuilderOptions = CommonOptions & {
		method?: string,
	};

	export function get(url: string, options?: RequestOptions): Promise<Response>;
	export function post(url: string, options?: RequestOptions): Promise<Response>;
	export function put(url: string, options?: RequestOptions): Promise<Response>;
	export function request(resource: string, method: string, options?: RequestOptions): Promise<Response>;
	export function request(resource: Request): Promise<Response>;

	export class Headers {
		constructor();
		constructor(headers: Headers);
		constructor(entries: HeaderEntries);
		constructor(object: HeadersObject);

		append(name: string, value: string);
		delete(name: string): boolean;
		get(name: string): Header | null;
		has(name: string): boolean;
		set(name: string, value: string);
	}

	export class Request {
		constructor(url: string, options?: RequestBuilderOptions);
		constructor(url: Request, options?: RequestBuilderOptions);
	}

	export class Response {
		constructor();

		get ok(): boolean;
		get status(): number;
		get statusText(): string;

		get bodyUsed(): boolean;
		get headers(): Headers;

		arrayBuffer(): Promise<ArrayBuffer>;
		text(): Promise<string>;
	}

	namespace Http {
		export {
			Body,
			RequestOptions,
			RequestBuilderOptions,

			get,
			post,
			put,
			request,

			Header,
			HeaderEntries,
			HeadersObject,

			Headers,
			Request,
			Response,
		};
	}

	export default Http;
}
