use clap::Parser;
use msgbox::IconType;
use std::error::Error;
use vraw_convert::convert_vraw;

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

    /// Specifies the output file name ex. video.mp4 (Folder path must exist)
    output: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::parse();

    if let Err(e) = convert_vraw(&config.input, config.output) {
        println!("Application error: {}", e);

        let err_msg: String = e.to_string();
        msgbox::create("vraw_convert", &err_msg, IconType::Info)?;
    }

    Ok(())
}
