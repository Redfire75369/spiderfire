pub(crate) const INDENT: &'static str = "  ";
const NEWLINE: &'static str = "\n";

pub(crate) fn indent(string: &String, indents: usize, initial: bool) -> String {
	if let Some(_) = string.find(NEWLINE) {
		let indent = INDENT.repeat(indents);
		if initial {
			str::replace(&(indent.clone() + string), NEWLINE, &("\n".to_owned() + &indent))
		} else {
			str::replace(&string, NEWLINE, &("\n".to_owned() + &indent))
		}
	} else {
		string.clone()
	}
}
