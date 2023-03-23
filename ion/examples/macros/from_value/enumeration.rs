use ion::FromValue;

#[derive(FromValue)]
#[repr(u8)]
enum Representation {
	Zero = 0,
	One = 1,
	Ten = 10,
}
