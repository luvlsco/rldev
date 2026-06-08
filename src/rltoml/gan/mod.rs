pub mod gan_convert;
pub mod gan_formatter;
pub mod gan_parser;

pub use gan_formatter::{format_gan_to_toml_error, gan_to_toml};
pub use gan_convert::{format_toml_to_gan_error, toml_to_gan};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct FrameAttrs {
	pub pattern: Option<i32>,
	pub x: Option<i32>,
	pub y: Option<i32>,
	pub time: Option<i32>,
	pub alpha: Option<i32>,
	pub other: Option<i32>,
}
