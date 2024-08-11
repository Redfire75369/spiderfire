declare type Transferable = ArrayBuffer;

declare interface StructuredSerializeOptions {
	transfer?: Transferable[]
}

declare function structuredClone<T>(value: T, options?: StructuredSerializeOptions): T;
