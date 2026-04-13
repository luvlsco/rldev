use clap::Parser;

#[derive(Parser, Debug)]
#[command(
	name = "RlToml",
	version = env!("CARGO_PKG_VERSION"),
	about = concat!(
		"convertor between RealLive auxiliary data formats and TOML\n",
		"Use --info to show more information about this program"),
	help_template = "{name} {version}: {about}\n\n{usage-heading} {usage}\n\n{all-args}\n",
	disable_help_flag = true,
	disable_version_flag = true,
)]
pub struct Args {
    #[arg(
        long = "help",
        help = "display this usage information"
    )]
    pub help: bool,

    #[arg(
        long = "version",
        help = "display RlToml version information"
    )]
    pub version: bool,

	#[arg(
		long = "info",
		help = "display detailed information about RlToml and its usage"
	)]
	pub info: bool,

    #[arg(short, long)]
    pub verbose: bool,
    
	#[arg(value_name = "FILES")]
    pub files: Vec<String>,
}

pub fn parse_args() -> Args {
    Args::parse()
}
