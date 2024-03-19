use serde::{self, Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(crate = "self::serde")]
pub struct Lock {
	pub token: String,
}
