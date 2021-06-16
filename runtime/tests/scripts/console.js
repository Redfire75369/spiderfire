console.log("Log", true, {});
console.info("Info", "Information", false, []);
console.warn("Warn", "Warning", 0, [1, "Array"]);
console.error("Error", undefined, {"key": "value"});
console.debug("Debug", "Debugging", null, {"key": {"value": "Object", "val": 1e3}});

// console.assert();
console.assert(true);
console.assert(false, "Assertion", true, 5);

console.clear();

function trace() {
	console.trace();
}
console.trace();
trace();

console.group("Indent 1:");
console.log("Indented");
console.groupCollapsed("Indent 2:");
console.log("More Indented");
console.groupEnd();
console.log("Less Indented");
console.groupEnd();
console.log("No Indent");

console.count("First Counter");
console.count("Second Counter");
console.count("First Counter");
console.countReset("Second Counter");
console.countReset("First Counter");
console.count("Second Counter");

console.time("Timer");
let val = 144;
for (let i = 0; i < 576; i++) {
	val += 12;
	if (i % 40 == 4) {
		console.timeLog("Timer", {val});
	}
}
console.timeEnd("Timer", ["Timer End."]);
