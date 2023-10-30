declare function setTimeout<T extends any[]>(callback: (...arguments: [...T]) => void, duration?: number, ...arguments: [...T]): number;

declare function setInterval<T extends any[]>(callback: (...arguments: [...T]) => void, duration?: number, ...arguments: [...T]): number;

declare function clearTimeout(id: number): void;

declare function clearInterval(id: number): void;

declare function queueMacrotask(callback: () => void): void;
