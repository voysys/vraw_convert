# vraw_convert

Convert Voysys .vraw files to other formats, using ffmpeg.

The tool can be run on the command line. Another way to use it is to drag and drop a .vraw file onto the application (or a shortcut to the application, tested on Windows).


## Dependencies

To use this tool, it is necessary to have ffmpeg installed system-wide. (On Windows ffmpeg.exe must be in PATH.)


## Overview

General procedure:

Read the raw video file frame by frame.
Pipe each raw frame to ffmpeg via stdin

---

* Determine frame rate
    * Parse frame index
        * Read time stamps
    * Average time between each frame

---

* Ignore header (skip first 16 bytes)
* Repeat until RecordingIndexHeader is reached (0xABCDFEED)
    * Parse RecordedFrameMetadata
        * Ignore magic, id, frameNo (skip 12 bytes)
        * Read width (4 bytes)
        * Read height (4 bytes)
        * Read format (4 bytes)
            * See VideoCaptureFormat code block
        * Ignore timestamp, receiveTimestamp (16 bytes)
        * Read total frame size (8 bytes)
    * Process frame data
        * Use ffmpeg with appropriate arguments based on frame width, height, format and size
    * Parse GenericMetadataHeader
        * Ignore magic (4 bytes)
        * Read genericMetadataSize (4 bytes)
    * Process generic metadata
        * Skip genericMetadataSize bytes
    * Parse GenericMetadataFooter
        * Ignore magic, genericMetadataSize (8 bytes)


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

Corresponding structs:
```cpp
#define RECORDING_MAGIC               0xFEEDFEED
#define RECORDING_FRAME_MAGIC         0xAAAAFEED
#define GENERIC_METADATA_HEADER_MAGIC 0xBACCDEEF
#define GENERIC_METADATA_FOOTER_MAGIC 0xBACCBEEF
#define RECORDING_INDEX_HEADER_MAGIC  0xABCDFEED
#define RECORDING_INDEX_FOOTER_MAGIC  0xDCBAFEED

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
