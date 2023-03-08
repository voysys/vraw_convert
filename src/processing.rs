use crate::parser::{parse_raw_frame, read_index, RecordingIndexEntry, VideoCaptureFormat};
use chrono::{DateTime, Local};
use mp4::{MediaConfig, Mp4Config, Mp4Sample, Mp4Writer, TrackConfig};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use zerocopy::AsBytes;

/// Function that converts a .vraw file to an .mp4 file.
/// NOTE: Currently only HEVC and MJPEG is supported!!!
///
/// input: path to .vraw file
///
/// output: name of the gengerated .mp4 file. If None is specified the file will
/// be named after the input and the time of generation.
pub fn convert_vraw(input: &String, output: Option<String>) -> Result<(), String> {
    let input_file = File::open(input).map_err(|_| "vraw_convert: failed to open file")?;

    let mut f = BufReader::new(input_file);

    let entries =
        read_index(&mut f).map_err(|e| format!("vraw_convert: failed to read index: {e}"))?;

    if entries.is_empty() {
        return Err("vraw_convert: index contains no frames".into());
    }

    let format = &entries
        .iter()
        .find_map(|entry| match parse_raw_frame(&mut f, entry).ok()?.format {
            VideoCaptureFormat::Stats => None,
            f => Some(f),
        })
        .ok_or(String::from("vraw_convert: unable to find a video frame"))?;

    let output = output.unwrap_or(derive_output_from_input(
        Path::new(input),
        &format,
        Local::now(),
    )?);
    let dst_file = File::create(output).map_err(|_| "vraw_convert: file creation failed")?;
    let writer = BufWriter::new(dst_file);
    match format {
        VideoCaptureFormat::H265 => extract_hevc_from_vraw(f, entries, writer)?,
        VideoCaptureFormat::Mjpeg => extract_mjpeg_from_vraw(f, entries, writer)?,
        e => unreachable!("unexpected format {:?}", e),
    }

    Ok(())
}

fn extract_hevc_from_vraw(
    mut f: BufReader<File>,
    entries: Vec<RecordingIndexEntry>,
    writer: BufWriter<File>,
) -> Result<(), String> {
    let config = Mp4Config {
        major_brand: str::parse("isom").unwrap(),
        minor_version: 512,
        compatible_brands: vec![str::parse("hev1").unwrap()],
        timescale: 1000, // This specifies milliseconds
    };

    let mut mp4_writer = Mp4Writer::write_start(writer, &config)
        .map_err(|_| "vraw_convert: failed to start writing mp4")?;

    // find first h265 frame
    let mut last_timestamp = 0;
    for entry in &entries {
        let frame =
            parse_raw_frame(&mut f, entry).map_err(|_| "vraw_convert: unable to read frame")?; // we discard the first frame for information about the video media
        match frame.format {
            VideoCaptureFormat::H265 => {
                mp4_writer
                    .add_track(&TrackConfig::from(MediaConfig::HevcConfig(
                        mp4::HevcConfig::default(),
                    )))
                    .map_err(|_| "vraw_convert: failed to add mp4 track")?;

                last_timestamp = frame.timestamp;

                break;
            }
            VideoCaptureFormat::Stats => {
                continue;
            }
            _ => return Err("VideoCaptureFormat not supported".into()),
        };
    }

    for entry in &entries {
        let raw_frame = parse_raw_frame(&mut f, entry);

        match raw_frame {
            Ok(frame) => {
                if frame.format == VideoCaptureFormat::Stats {
                    continue;
                }

                let delta_t = (frame.timestamp - last_timestamp) as f64 * 1e-6; // duration in milliseconds of the frame
                let video_sample = Mp4Sample {
                    start_time: frame.timestamp as u64,
                    duration: delta_t.round() as u32, // round to nearest millisecond
                    rendering_offset: 0,
                    is_sync: false,
                    bytes: mp4::Bytes::copy_from_slice(frame.raw_data.as_bytes()),
                };

                mp4_writer
                    .write_sample(1, &video_sample)
                    .map_err(|_| "vraw_convert: failed to write sample")?;

                last_timestamp = frame.timestamp;
            }
            Err(_) => {
                // Here, we don't have a valid frame (we most likely reached the end of the recording)
                break;
            }
        }
    }

    mp4_writer
        .write_end()
        .map_err(|_| "vraw_convert: failed to end mp4 writing")?;

    Ok(())
}

fn extract_mjpeg_from_vraw(
    mut f: BufReader<File>,
    entries: Vec<RecordingIndexEntry>,
    mut writer: BufWriter<File>,
) -> Result<(), String> {
    for entry in entries {
        let frame = parse_raw_frame(&mut f, &entry)
            .map_err(|e| format!("mjpeg_convert: failed to parse frame: {:?}", e))?;
        if frame.format != VideoCaptureFormat::Mjpeg {
            eprintln!(
                "mjpeg_convert: skipping frame in {:?} format...",
                frame.format
            );
        }
        writer
            .write(&frame.raw_data)
            .map_err(|e| format!("mjpeg_convert: failed to write frame to output: {:?}", e))?;
    }
    Ok(())
}

fn derive_output_from_input(
    input_path: &Path,
    format: &VideoCaptureFormat,
    timestamp: DateTime<Local>,
) -> Result<String, String> {
    let output_file_name = input_path.file_name().unwrap().to_str().unwrap();

    let extension = match format {
        VideoCaptureFormat::H265 => "mp4",
        VideoCaptureFormat::Mjpeg => "mjpeg",
        _ => return Err("derive_output_name: unsupported video format")?,
    };

    let output_file_name = format!(
        "{}_{}.{}",
        output_file_name.trim_end_matches(".vraw"),
        timestamp.format("%Y-%m-%dT%H_%M_%S"),
        extension,
    );

    Ok(input_path
        .ancestors()
        .nth(1)
        .unwrap()
        .join(output_file_name)
        .to_string_lossy()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    #[test]
    pub fn derive_output_from_input_same_folder_mp4() {
        let input = Path::new("/path/to/raw_recording/recording.vraw");
        let timestamp = Local.ymd(2022, 03, 07).and_hms(20, 50, 0);

        let output = derive_output_from_input(input, &VideoCaptureFormat::H265, timestamp).unwrap();
        assert_eq!(
            "/path/to/raw_recording/recording_2022-03-07T20_50_00.mp4",
            output
        );
    }

    #[test]
    pub fn derive_output_from_input_same_folder_mjpeg() {
        let input = Path::new("/path/to/raw_recording/recording.vraw");
        let timestamp = Local.ymd(2022, 03, 07).and_hms(20, 50, 0);

        let output =
            derive_output_from_input(input, &VideoCaptureFormat::Mjpeg, timestamp).unwrap();
        assert_eq!(
            "/path/to/raw_recording/recording_2022-03-07T20_50_00.mjpeg",
            output
        );
    }
}
