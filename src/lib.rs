mod parser;
mod processing;

pub use processing::convert_vraw_to_mp4;

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::Path,
    };

    #[test]
    fn try_convert_h265() {
        crate::processing::convert_vraw_to_mp4(
            &"assets/h265.vraw".to_string(),
            Some(String::from("assets/h265.mp4")),
        )
        .unwrap();

        let files_equal = compare_bytes_in_files(
            Path::new("assets/h265.mp4"),
            Path::new("assets/golden/h265.mp4"),
        )
        .unwrap();
        assert!(files_equal);
    }

    #[test]
    fn try_convert_no_video_alignment_data() {
        crate::processing::convert_vraw_to_mp4(
            &"assets/no_output_alignment.vraw".to_string(),
            Some(String::from("assets/no_output_alignment.mp4")),
        )
        .unwrap();

        let files_equal = compare_bytes_in_files(
            Path::new("assets/no_output_alignment.mp4"),
            Path::new("assets/golden/no_output_alignment.mp4"),
        )
        .unwrap();
        assert!(files_equal);
    }

    fn compare_bytes_in_files(pa: &Path, pb: &Path) -> Result<bool, String> {
        let fa = File::open(pa).map_err(|e| format!("failed to open {:?}: {}", pa, e))?;
        let fb = File::open(pb).map_err(|e| format!("failed to open {:?}: {}", pb, e))?;

        if fa.metadata().unwrap().len() != fb.metadata().unwrap().len() {
            return Ok(false);
        }

        let ra = BufReader::new(fa);
        let rb = BufReader::new(fb);

        Ok(ra.bytes().zip(rb.bytes()).all(|bs| match bs {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }))
    }
}
