use serde::{self, Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(crate = "self::serde")]
pub struct User {
	pub id: i32,
	pub name: String,
	pub email: String,
}
