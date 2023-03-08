# vraw_convert

Convert Voysys .vraw files to .mp4 or .mpjpeg video files

## Usage:
Clone the repo:
```rust
git clone git@github.com:voysys/vraw_convert.git
```
Build the project:
```rust
cargo build --release
```
Execute the binary with the input .vraw and/or the ouput .mp4:
```rust
./target/release/vraw_convert.exe input.vraw output.mp4
```

## Issues
- The generated MP4 cannot be played in windows media player. VLC can be used to play the extracted .mp4.
- Folder path to the output.mp4 need to exist.
- The generated MJPEG file cannot be played in VLC.
    - `ffplay` can be used to playback the video
    - `ffmpeg -i IN.mjpeg OUT.mp4` to convert the extracted MJPEG file
	to a MP4 file that can be opened and played back in VLC.

## Voysys vraw video format description

The following tables describe the layout of the raw recording format:

| Header content | Size [bytes] |
| -------------- | ------------ |
| Metadata       | 16           |


| Frame content         | Size [bytes] |
| --------------------- | ------------ |
| RecordedFrameMetadata | 48           |
| Raw data              | Variable     |
| GenericMetadataHeader | 8            |
| GenericMetadata       | Variable     |
| GenericMetadataFooter | 8            |


| Footer content | Size [bytes] |
| -------------- | ------------ |
| Alignment data | 7            |

Corresponding structs:
```cpp
#define RECORDING_MAGIC               0xFEEDFEED
#define RECORDING_FRAME_MAGIC         0xAAAAFEED
#define GENERIC_METADATA_HEADER_MAGIC 0xBACCDEEF
#define GENERIC_METADATA_FOOTER_MAGIC 0xBACCBEEF
#define RECORDING_INDEX_HEADER_MAGIC  0xABCDFEED
#define RECORDING_INDEX_FOOTER_MAGIC  0xDCBAFEED
#define VIDEO_PLACEMENT_METADATA_MAGIC_1 0x00;
#define VIDEO_PLACEMENT_METADATA_MAGIC_2 0x00;
#define VIDEO_PLACEMENT_METADATA_MAGIC_3 0x00;
#define VIDEO_PLACEMENT_METADATA_MAGIC_4 0x56;
#define VIDEO_PLACEMENT_METADATA_MAGIC_5 0x4A;

struct RecordingMetadata {
    u32 magic = RECORDING_MAGIC;
    u32 unixEpocTimeRelativeNsec;
    u64 unixEpocTimeSec;
};

struct RecordedFrameMetadata {
    u32 magic = RECORDING_FRAME_MAGIC;                        // This value will always be 0xAAAAFEED
    i32 id = 0;                                               // This value corresponds to the stream id in the pv file
    i32 frameNo = 0;                                          // The frame number of this frame
    i32 width = 0;
    i32 height = 0;
    i32 format = static_cast<i32>(VideoCaptureFormat::Raw);   // What format is this frame in
    i64 timestamp = 0;                                        // The timestamp from the capture system of this frame if any
    i64 receiveTimestamp = 0;                                 // The timestamp when this frame was recived
    i64 size = 0;                                             // The size of the following frame
};

struct GenericMetadataHeader {
    u32 magic = GENERIC_METADATA_HEADER_MAGIC;                // This value will always be 0xBACCDEEF
    u32 genericMetadataSize = 0;                              // Size of the generic metadata block
};

struct GenericMetadataFooter {
    u32 magic = GENERIC_METADATA_FOOTER_MAGIC;                // This value will always be 0xBACCBEEF
    u32 genericMetadataSize = 0;                              // Size of the generic metadata block
};

struct RecordingIndexHeader {
    u32 magic = RECORDING_INDEX_HEADER_MAGIC;                 // This value will always be 0xABCDFEED
    u32 _padding = 0;
};

struct VideoPlacementMetadataFooter {
    metadata_size: 0,
    magic_1 = VIDEO_PLACEMENT_METADATA_MAGIC_1,
    magic_2 = VIDEO_PLACEMENT_METADATA_MAGIC_2,
    magic_3 = VIDEO_PLACEMENT_METADATA_MAGIC_3,
    magic_4 = VIDEO_PLACEMENT_METADATA_MAGIC_4,
    magic_5 = VIDEO_PLACEMENT_METADATA_MAGIC_5,
}
```

```cpp
enum class VideoCaptureFormat : i32 {
    Rgb     =  0,
    Bgr     =  1,
    Yuv     =  2,
    Nv12    =  3,
    Yuyv    =  4,
    Uyvy    =  5,
    Raw     =  6,
    Mono16  =  7,
    Raw16   =  8,
    Mono8   =  9,
    Invalid = -1,
};
```
