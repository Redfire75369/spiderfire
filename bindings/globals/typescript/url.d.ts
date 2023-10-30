declare class URL {
	constructor(url: string, base?: string);

	get href(): string;

	set href(href: string);

	get origin(): string;

	get protocol(): string;

	set protocol(protocol: string);

	get username(): string;

	set username(username: string);

	get password(): string | null;

	set password(password: string);

	get host(): string | null;

	set host(host: string);

	get hostname(): string | null;

	set hostname(hostname: string);

	get port(): number | null;

	set port(port: number);

	get pathname(): string;

	set pathname(path: string);

	get search(): string | null;

	set search(string: string);

	get searchParams(): URLSearchParams;

	get hash(): string | null;

	set hash(hash: string);

	static canParse(url: string, base?: string): boolean;

	toString(): string;

	toJSON(): string;
}

declare class URLSearchParams implements Iterable<[string, string]> {
	constructor(init?: [string, string][] | Record<string, string> | string);

	get size(): number;

	append(name: string, value: string): void;

	delete(name: string, value?: string): void;

	get(name: string): string | null;

	getAll(name: string): string[];

	has(name: string, value?: string): boolean;

	set(name: string, value: string): void;

	sort(): void;

	[Symbol.iterator](): Iterator<[string, string]>;

	toString(): string;
}

