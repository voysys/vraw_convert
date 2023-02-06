use byteorder::LittleEndian;
use static_assertions::const_assert_eq;
use std::{
    convert::TryFrom,
    error::Error,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    mem::{self, size_of},
};
use zerocopy::{AsBytes, FromBytes, LayoutVerified, Unaligned};

const GENERIC_METADATA_HEADER_MAGIC: u32 = 0xBACCDEEF;
const RECORDING_FRAME_MAGIC: u32 = 0xAAAAFEED;
const RECORDING_INDEX_FOOTER_MAGIC: u32 = 0xDCBAFEED;

const VIDEO_PLACEMENT_METADATA_MAGIC_1: u8 = 0x00;
const VIDEO_PLACEMENT_METADATA_MAGIC_2: u8 = 0x00;
const VIDEO_PLACEMENT_METADATA_MAGIC_3: u8 = 0x00;
const VIDEO_PLACEMENT_METADATA_MAGIC_4: u8 = 0x56;
const VIDEO_PLACEMENT_METADATA_MAGIC_5: u8 = 0x4A;

type I32 = zerocopy::I32<LittleEndian>;
type I64 = zerocopy::I64<LittleEndian>;
type U16 = zerocopy::U16<LittleEndian>;
type U32 = zerocopy::U32<LittleEndian>;
type U64 = zerocopy::U64<LittleEndian>;

#[derive(Debug, Clone, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
struct RecordingMetadata {
    magic: U32,
    unix_epoch_time_relative_nsec: U32,
    unix_epoch_time_sec: U64,
}

const_assert_eq!(mem::size_of::<RecordingMetadata>(), 16);

#[derive(Debug, Clone, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
struct RecordedFrameMetadata {
    magic: U32,
    id: I32,
    padding: I32,
    width: I32,
    height: I32,
    format: I32,
    timestamp: I64,
    receive_timestamp: I64,
    size: I64,
}

const_assert_eq!(mem::size_of::<RecordedFrameMetadata>(), 48);

#[derive(Debug, Clone, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
struct GenericMetadataHeader {
    magic: U32,
    generic_metadata_size: U32,
}

const_assert_eq!(mem::size_of::<GenericMetadataHeader>(), 8);

#[derive(Debug, Clone, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
struct RecordingIndexHeader {
    magic: U32,
    padding: U32,
}

const_assert_eq!(mem::size_of::<RecordingIndexHeader>(), 8);

#[derive(Debug, Clone, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
pub struct RecordingIndexEntry {
    offset: I64,
    receive_timestamp: I64,
}

const_assert_eq!(mem::size_of::<RecordingIndexEntry>(), 16);

#[derive(Debug, Clone, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
struct RecordingIndexFooter {
    magic: U32,
    frame_count: U32,
}

const_assert_eq!(mem::size_of::<RecordingIndexFooter>(), 8);

#[derive(Debug, Clone, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
struct VideoPlacementMetadataFooter {
    metadata_size: U16,
    magic_1: u8,
    magic_2: u8,
    magic_3: u8,
    magic_4: u8,
    magic_5: u8,
}

const_assert_eq!(mem::size_of::<VideoPlacementMetadataFooter>(), 7);

#[derive(Debug, Clone)]
pub struct FrameInfo {
    pub resolution: String,
    pub format: VideoCaptureFormat,
    pub raw_data: Vec<u8>,
    pub timestamp: i64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i32)]
pub enum VideoCaptureFormat {
    Rgb = 0,
    Bgr = 1,
    Yuv = 2,
    Nv12 = 3,
    Yuyv = 4,
    Uyvy = 5,
    Raw = 6,
    Mono16 = 7,
    Raw16 = 8,
    Mono8 = 9,
    H264 = -4601,
    H265 = -4602,
    Mjpeg = -4603,
    Stats = -4701,
}

impl VideoCaptureFormat {
    pub fn is_coded(&self) -> bool {
        matches!(
            self,
            VideoCaptureFormat::H264 | VideoCaptureFormat::H265 | VideoCaptureFormat::Mjpeg
        )
    }
}

impl TryFrom<i32> for VideoCaptureFormat {
    type Error = Box<dyn Error>;

    fn try_from(format: i32) -> Result<Self, Self::Error> {
        match format {
            0 => Ok(VideoCaptureFormat::Rgb),
            1 => Ok(VideoCaptureFormat::Bgr),
            2 => Ok(VideoCaptureFormat::Yuv),
            3 => Ok(VideoCaptureFormat::Nv12),
            4 => Ok(VideoCaptureFormat::Yuyv),
            5 => Ok(VideoCaptureFormat::Uyvy),
            6 => Ok(VideoCaptureFormat::Raw),
            7 => Ok(VideoCaptureFormat::Mono16),
            8 => Ok(VideoCaptureFormat::Raw16),
            9 => Ok(VideoCaptureFormat::Mono8),
            -4601 => Ok(VideoCaptureFormat::H264),
            -4602 => Ok(VideoCaptureFormat::H265),
            -4603 => Ok(VideoCaptureFormat::Mjpeg),
            -4701 => Ok(VideoCaptureFormat::Stats),
            _ => Err(format!("Unknown video capture format {}", format).into()),
        }
    }
}

fn parse_recording_index_footer(bytes: &[u8]) -> Result<&RecordingIndexFooter, Box<dyn Error>> {
    LayoutVerified::<&[u8], RecordingIndexFooter>::new_unaligned(bytes)
        .ok_or_else(|| "Failed to parse RecordingIndexFooter".into())
        .map(|lv| lv.into_ref())
        .and_then(|res| {
            if res.magic.get() == RECORDING_INDEX_FOOTER_MAGIC {
                Ok(res)
            } else {
                Err("Magic does not match".into())
            }
        })
}

fn parse_recording_index_entry(bytes: &[u8]) -> Result<&RecordingIndexEntry, Box<dyn Error>> {
    LayoutVerified::<&[u8], RecordingIndexEntry>::new_unaligned(bytes)
        .ok_or_else(|| "Failed to parse RecordingIndexEntry".into())
        .map(|lv| lv.into_ref())
}

fn parse_recorded_frame_metadata(bytes: &[u8]) -> Result<&RecordedFrameMetadata, Box<dyn Error>> {
    LayoutVerified::<&[u8], RecordedFrameMetadata>::new_unaligned(bytes)
        .ok_or_else(|| "Failed to parse RecordedFrameMetadata".into())
        .map(|lv| lv.into_ref())
        .and_then(|res| {
            if res.magic.get() == RECORDING_FRAME_MAGIC {
                Ok(res)
            } else {
                Err("Magic does not match".into())
            }
        })
}

fn parse_generic_metadata_header(bytes: &[u8]) -> Result<&GenericMetadataHeader, Box<dyn Error>> {
    LayoutVerified::<&[u8], GenericMetadataHeader>::new_unaligned(bytes)
        .ok_or_else(|| "Failed to parse GenericMetadataHeader".into())
        .map(|lv| lv.into_ref())
        .and_then(|res| {
            if res.magic.get() == GENERIC_METADATA_HEADER_MAGIC {
                Ok(res)
            } else {
                Err("Magic does not match".into())
            }
        })
}

fn parse_video_placement_footer(
    bytes: &[u8],
) -> Result<&VideoPlacementMetadataFooter, Box<dyn Error>> {
    LayoutVerified::<&[u8], VideoPlacementMetadataFooter>::new_unaligned(bytes)
        .ok_or_else(|| "Failed to parse VideoPlacementMetadataFooter".into())
        .map(|lv| lv.into_ref())
        .and_then(|res| {
            if res.magic_1 == VIDEO_PLACEMENT_METADATA_MAGIC_1
                && res.magic_2 == VIDEO_PLACEMENT_METADATA_MAGIC_2
                && res.magic_3 == VIDEO_PLACEMENT_METADATA_MAGIC_3
                && res.magic_4 == VIDEO_PLACEMENT_METADATA_MAGIC_4
                && res.magic_5 == VIDEO_PLACEMENT_METADATA_MAGIC_5
            {
                Ok(res)
            } else {
                Err("Magic does not match".into())
            }
        })
}

pub fn read_index(f: &mut BufReader<File>) -> Result<Vec<RecordingIndexEntry>, Box<dyn Error>> {
    f.seek(SeekFrom::End(
        -(mem::size_of::<RecordingIndexFooter>() as i64),
    ))?;

    let mut index_footer_bytes: [u8; mem::size_of::<RecordingIndexFooter>()] =
        [0; mem::size_of::<RecordingIndexFooter>()];
    f.read_exact(&mut index_footer_bytes).unwrap();

    let footer = parse_recording_index_footer(&index_footer_bytes)?;

    f.seek(SeekFrom::End(
        -((mem::size_of::<RecordingIndexFooter>()
            + footer.frame_count.get() as usize * mem::size_of::<RecordingIndexEntry>())
            as i64),
    ))?;

    // At the first frame now
    let mut res = Vec::with_capacity(footer.frame_count.get() as _);

    for _ in 0..footer.frame_count.get() {
        let mut index_entry_bytes: [u8; mem::size_of::<RecordingIndexEntry>()] =
            [0; mem::size_of::<RecordingIndexEntry>()];
        f.read_exact(&mut index_entry_bytes)?;

        let entry = parse_recording_index_entry(&index_entry_bytes)?;

        res.push(entry.to_owned());
    }

    Ok(res)
}

pub fn parse_raw_frame(
    f: &mut BufReader<File>,
    entry: &RecordingIndexEntry,
) -> Result<FrameInfo, Box<dyn Error>> {
    f.seek(SeekFrom::Start(entry.offset.get() as _))?;

    // ------------------------------------------------------------------------
    // Parse header
    let mut recorded_frame_metadata_bytes: [u8; mem::size_of::<RecordedFrameMetadata>()] =
        [0; mem::size_of::<RecordedFrameMetadata>()];
    f.read_exact(&mut recorded_frame_metadata_bytes)?;

    let recorded_frame_metadata =
        parse_recorded_frame_metadata(&recorded_frame_metadata_bytes[..])?;

    if recorded_frame_metadata.size.get() <= 0 {
        return Err("Frame size not parsed correctly.".into());
    }

    let format = VideoCaptureFormat::try_from(recorded_frame_metadata.format.get())?;

    if format.is_coded() {
        if recorded_frame_metadata.width.get() != 0 && recorded_frame_metadata.height.get() != 0 {
            return Err("Frame width and height not parsed correctly.".into());
        }
    } else if format != VideoCaptureFormat::Stats
        && (recorded_frame_metadata.width.get() <= 0 || recorded_frame_metadata.height.get() <= 0)
    {
        return Err("Frame width and height not parsed correctly.".into());
    }

    // ------------------------------------------------------------------------
    // Read frame data
    let mut raw_frame_data: Vec<u8> = vec![0; recorded_frame_metadata.size.get() as usize];
    f.read_exact(&mut raw_frame_data)?;

    // ------------------------------------------------------------------------
    // Parse VideoPlacementMetadataFooter
    let frame_data: Vec<u8>;
    if format != VideoCaptureFormat::Stats {
        let mut offset = 0;

        loop {
            // Loop from the end to try and match the video placement magic(s)
            if let Ok(video_placement_footer) = parse_video_placement_footer(
                &raw_frame_data[(raw_frame_data.len()
                    - size_of::<VideoPlacementMetadataFooter>()
                    - offset)..(raw_frame_data.len() - offset)],
            ) {
                frame_data = raw_frame_data[..(raw_frame_data.len()
                    - video_placement_footer.clone().metadata_size.get() as usize
                    - size_of::<VideoPlacementMetadataFooter>())]
                    .to_vec();

                break;
            } else {
                if offset > 10 {
                    // If the end has to be looped more than 10 times then it probably do not have alignment data
                    frame_data = raw_frame_data.clone();
                    break;
                }

                offset += 1;
            }
        }
    } else {
        frame_data = raw_frame_data.clone();
    }

    // ------------------------------------------------------------------------
    // Parse generic metadata header
    let mut generic_metadata_header_or_footer_data: [u8; 8] = [0; 8];
    f.read_exact(&mut generic_metadata_header_or_footer_data)?;
    let generic_metadata_header =
        parse_generic_metadata_header(&generic_metadata_header_or_footer_data[..])?;

    // ------------------------------------------------------------------------
    // Parse generic metadata
    let mut generic_metadata_data: Vec<u8> =
        vec![0; generic_metadata_header.generic_metadata_size.get() as usize];
    f.read_exact(&mut generic_metadata_data)?;

    // ------------------------------------------------------------------------
    // Parse generic metadata footer
    f.read_exact(&mut generic_metadata_header_or_footer_data)?;

    let resolution = recorded_frame_metadata.width.to_string()
        + "x"
        + &recorded_frame_metadata.height.to_string();

    Ok(FrameInfo {
        resolution,
        format,
        timestamp: recorded_frame_metadata.receive_timestamp.get(),
        raw_data: frame_data,
    })
}
