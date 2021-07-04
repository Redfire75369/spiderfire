export var a = 0;
export let b = 8;
export const c = "SpiderMonkey";

export function f(p) {
	console.trace(p);
}

const module = {
	a,
	f
};

export default module;
