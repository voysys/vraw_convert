use crate::{
    parser::{
        parse_and_discard_recording_metadata, parse_raw_frame, FrameInfo, VideoCaptureFormat,
    },
    Config,
};
use chrono::Local;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::{error::Error, fs::File};
use zerocopy::AsBytes;

use mp4::{HevcConfig, Mp4Config, Mp4Sample, Mp4Writer, TrackConfig};
use std::io::Cursor;

const NR_FRAMES_TO_DISCARD: u32 = 10;

struct FFmpegSetupInfo<'a> {
    output_filename: String,
    format: VideoCaptureFormat,
    resolution: String,
    config: &'a Config,
}

struct FFmpegInstance<'a> {
    info: FFmpegSetupInfo<'a>,
    stdin_handle: ChildStdin,
}

fn setup_ffmpeg_pipe(setup_info: &FFmpegSetupInfo) -> Result<Child, Box<dyn Error>> {
    if setup_info.format.is_coded() {
        Command::new("ffmpeg")
            .args(&[
                "-f",
                setup_info.format.ffmpeg_demuxer(),
                "-framerate",
                &setup_info.config.framerate,
                "-i",
                "pipe:0",
                "-c:v",
                setup_info.format.ffmpeg_codec(),
                &setup_info.output_filename,
            ])
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|err| err.into())
    } else {
        Command::new("ffmpeg")
            .args(&[
                "-f",
                setup_info.format.ffmpeg_demuxer(),
                "-s",
                &setup_info.resolution,
                "-r",
                &setup_info.config.framerate,
                "-pix_fmt",
                setup_info.format.ffmpeg_pix_fmt(),
                "-i",
                "pipe:0",
                "-c:v",
                setup_info.format.ffmpeg_codec(),
                "-preset",
                &setup_info.config.preset,
                "-crf",
                &setup_info.config.crf,
                &setup_info.output_filename,
            ])
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|err| err.into())
    }
}

fn write_to_ffmpeg(handle: &mut ChildStdin, data: &[u8]) -> Result<(), Box<dyn Error>> {
    if let Err(e) = handle.write_all(data) {
        return Err(format!("Couldn't write to ffmpeg stdin: {}", e).into());
    }
    Ok(())
}

/// Initializes the ffmpeg pipeline
///
/// It first reads two frames to determine the framerate
/// those two frames are then written to the ffmpeg process
///
fn initialize_frame_processing<'a>(
    f: &mut dyn BufRead,
    config: &'a Config,
    output_filename: &str,
) -> Result<FFmpegInstance<'a>, Box<dyn Error>> {
    let first_frame = parse_raw_frame(f)?;

    let ffmpeg_setup_info = FFmpegSetupInfo {
        output_filename: format!("{}.mp4", output_filename),
        format: first_frame.format,
        resolution: first_frame.resolution,
        config,
    };

    let ffmpeg_process = setup_ffmpeg_pipe(&ffmpeg_setup_info)?;
    let mut handle = match ffmpeg_process.stdin {
        None => return Err("Error setting up ffmpeg pipeline".into()),
        Some(handle) => handle,
    };

    write_to_ffmpeg(&mut handle, &first_frame.raw_data[..])?;

    Ok(FFmpegInstance {
        info: ffmpeg_setup_info,
        stdin_handle: handle,
    })
}

fn discard_frames(f: &mut dyn BufRead, n: u32) -> Result<(), Box<dyn Error>> {
    for _ in 0..n {
        if parse_raw_frame(f).is_err() {
            return Err("Error discarding frames".into());
        }
    }

    Ok(())
}

fn is_same_frame_format(frame_a: &FrameInfo, frame_b: &FFmpegSetupInfo) -> bool {
    frame_a.resolution == frame_b.resolution && frame_a.format == frame_b.format
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let input_file = File::open(&config.input)?;
    let output = config.output.clone().unwrap_or(format!(
        "{}_{}_part_",
        config.input.trim_end_matches(".vraw"),
        Local::now().format("%Y-%m-%dT%H_%M_%S")
    ));

    let mut f = BufReader::new(input_file);

    parse_and_discard_recording_metadata(&mut f)?;

    // // The "0" gives the first video in the recording the file ending "part_0"
    // let mut ffmpeg_instance =
    //     initialize_frame_processing(&mut f, &config, &format!("{}{}", &output, 0))?;

    // let mut unique_video_counter = 0;

    // loop {
    //     let raw_frame = parse_raw_frame(&mut f);

    //     match raw_frame {
    //         Ok(frame) => {
    //             if frame.format == VideoCaptureFormat::Stats {
    //                 continue;
    //             }

    //             if is_same_frame_format(&frame, &ffmpeg_instance.info) {
    //                 ffmpeg_instance
    //                     .stdin_handle
    //                     .write_all(&frame.raw_data[..])?;
    //             } else {
    //                 // Valid frame but with a new resolution or format. Need to reinitialize the
    //                 // pipeline

    //                 // Frames most likely will have artefacts when you switch to a new
    //                 // resolution or pixel format. We discard a few to prevent weird errors
    //                 discard_frames(&mut f, NR_FRAMES_TO_DISCARD)?;
    //                 unique_video_counter += 1;
    //                 let new_filename = format!("{}{}", &output, &unique_video_counter);
    //                 ffmpeg_instance = initialize_frame_processing(&mut f, &config, &new_filename)?;
    //             }
    //         }
    //         // Here, we don't have a valid frame (we most likely reached the end of the recording)
    //         Err(_) => {
    //             break;
    //         }
    //     }
    // }

    let config = Mp4Config {
        major_brand: str::parse("isom").unwrap(),
        minor_version: 512,
        compatible_brands: vec![str::parse("hev1").unwrap()],
        timescale: 1000,
    };

    let dst_file = File::create("hej.mp4")?;
    let writer = BufWriter::new(dst_file);

    let mut mp4_writer = Mp4Writer::write_start(writer, &config)?;
    mp4_writer.add_track(&TrackConfig {
        track_type: mp4::TrackType::Video,
        timescale: 60, // TODO: Calculate this from raw rec
        language: "english".to_string(),
        media_conf: mp4::MediaConfig::HevcConfig(HevcConfig {
            width: 1920,
            height: 1080,
        }),
    })?;

    let mut _unique_video_counter = 0;
    let mut last_timestamp = 0;
    loop {
        let raw_frame = parse_raw_frame(&mut f);

        match raw_frame {
            Ok(frame) => {
                if frame.format == VideoCaptureFormat::Stats {
                    continue;
                }

                let delta_t = (frame.timestamp - last_timestamp) as f64 * 60e-9; // TODO: is this the correct value?
                let test = Mp4Sample {
                    start_time: frame.timestamp as u64,
                    duration: delta_t.round() as u32, // TODO: is this the correct value?
                    rendering_offset: 0,
                    is_sync: false,
                    bytes: mp4::Bytes::copy_from_slice(frame.raw_data.as_bytes()),
                };

                mp4_writer.write_sample(1, &test)?;

                last_timestamp = frame.timestamp;

                // println!("{}", delta_t);
            }
            Err(_) => {
                break;
            }
        }
    }

    mp4_writer.write_end();

    Ok(())
}
