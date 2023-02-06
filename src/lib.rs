mod parser;
mod processing;

pub use processing::convert_vraw_to_mp4;

#[cfg(test)]
mod tests {
    #[test]
    fn try_convert_h265() {
        crate::processing::convert_vraw_to_mp4(&"assets/h265.vraw".to_string(), None).unwrap();
    }

    #[test]
    fn try_convert_no_video_alignment_data() {
        crate::processing::convert_vraw_to_mp4(
            &"assets/no_output_alignment.vraw".to_string(),
            None,
        )
        .unwrap();
    }
}
