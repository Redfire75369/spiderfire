pub(crate) const INDENT: &str = "  ";
const NEWLINE: &str = "\n";

pub(crate) fn indent(string: &str, indents: usize, initial: bool) -> String {
	if string.contains(NEWLINE) {
		let indent = INDENT.repeat(indents);
		if initial {
			str::replace(&(indent.clone() + string), NEWLINE, &("\n".to_owned() + &indent))
		} else {
			str::replace(&string, NEWLINE, &("\n".to_owned() + &indent))
		}
	} else {
		string.to_string()
	}
}
