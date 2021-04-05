use spiderfire::Config;
use spiderfire::CONFIG;
use spiderfire::run;

#[test]
fn main_js() {
	let config = Config::initialise(true, true).unwrap();
	CONFIG.set(config).unwrap();
	run::run(&String::from("./tests/main.js"));
}

#[test]
fn module_js() {
	let config = Config::initialise(true, true).unwrap();
	CONFIG.set(config).unwrap();
	run::run(&String::from("./tests/module.js"));
}
