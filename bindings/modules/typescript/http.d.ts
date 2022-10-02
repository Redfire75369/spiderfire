declare module "http" {
	export type Header = string | string[];
	export interface Headers {
		[key: string]: Header,
	}

	type TypedArray = Int8Array | Int16Array | Int32Array
		| Uint8Array | Uint8ClampedArray | Uint16Array | Uint32Array
		| Float32Array | Float64Array;
	export type Body = string | String | ArrayBuffer | TypedArray | DataView;

	interface CommonOptions {
		setHost?: boolean,
		headers?: Headers,
		uniqueHeaders?: Headers,
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
			Header,
			Headers,
			Body,
			RequestOptions,
			RequestBuilderOptions,

			get,
			post,
			put,
			request,

			Request,
			Response,
		};
	}

	export default Http;
}
