console.log("Log", true, false);
console.info("Info", "Information", 0, -4e8, 2 ** 16);
console.warn("Warn", "Warning", undefined, [], [3, false, "String"]);
console.error("Error", null, {}, {"key": "value"}, /^\d{8}$/gi);
console.debug("Debug", {"key": {"obj": "Object", "date": new Date()}}, function debug() {});

console.assert();
console.assert(true);
console.assert(false, "Assertion:", true, "Time -", new Date());

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

console.count();
console.count("First Counter");
console.count("Second Counter");
console.count("First Counter");
console.countReset("Second Counter");
console.count();
console.countReset("First Counter");
console.count("Second Counter");
console.countReset();

console.time();
console.time("Timer");
let val = 144;
for (let i = 0; i < 576; i++) {
	val += 12;
	if (i % 40 === 4) {
		console.timeLog("Timer", {val});
	}
	if (i % 20 === 48) {
		console.timeLog();
	}
}
console.timeEnd();
console.timeEnd("Timer");
