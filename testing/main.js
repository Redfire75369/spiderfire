import fs from "fs";

function x() {
	function y() {
		console.trace();
		console.log(fs.readBinary("testing/module.js"));
	}

	y();
}

x();
