use clap::Parser;
use msgbox::IconType;
use processing::run;
use std::error::Error;

mod parser;
mod processing;

#[derive(Parser)]
#[clap(
    name = "vraw_convert",
    version = "0.2",
    author = "Voysys AB",
    about = "Converts Voysys .vraw recordings to other formats, using ffmpeg"
)]
pub struct Config {
    /// Specifies the raw input file
    #[clap(default_value = "in.vraw")]
    input: String,

    /// Specifies the framerate
    #[clap(short, long, default_value = "30")]
    framerate: String,

    /// Specifies the x264 ffmpeg preset to use
    #[clap(short, long, default_value = "veryfast")]
    preset: String,

    /// Specifies the x264 crf value
    #[clap(long, default_value = "23")]
    crf: String,

    /// Specifies the output file name
    output: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::parse();

    if let Err(e) = run(config) {
        println!("Application error: {}", e);

        let err_msg: String = e.to_string();
        msgbox::create("vraw_convert", &err_msg, IconType::Info)?;
    }

    Ok(())
}
