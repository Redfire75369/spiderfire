import module, {a, b, c, f} from "../scripts/module-export.js";

console.assert(a === 0);
console.assert(b === 8);
console.assert(c === "SpiderMonkey");
f(a);

console.assert(module.a === 0);
module.f(a);

console.log(10000000);
