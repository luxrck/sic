use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sic_core::image;
use sic_core::image::AnimationDecoder;

/// Load an image using a reader.
/// All images are currently loaded from memory.
pub fn load_image<R: Read>(
    reader: &mut R,
    config: &ImportConfig,
) -> ImportResult<image::DynamicImage> {
    let buffer = load(reader)?;

    if starts_with_gif_magic_number(&buffer) {
        load_gif(&buffer, config.selected_frame)
    } else {
        image::load_from_memory(&buffer).map_err(From::from)
    }
}

/// Result which is returned for operations within this module.
type ImportResult<T> = Result<T, ImportError>;

/// Constructs a reader which reads from the stdin.
pub fn stdin_reader() -> ImportResult<Box<dyn Read>> {
    Ok(Box::new(BufReader::new(std::io::stdin())))
}

/// Constructs a reader which reads from a file path.
pub fn file_reader<P: AsRef<Path>>(path: P) -> ImportResult<Box<dyn Read>> {
    Ok(Box::new(BufReader::new(File::open(path)?)))
}

// Let the reader store the raw bytes into a buffer.
fn load<R: Read>(reader: &mut R) -> ImportResult<Vec<u8>> {
    let mut buffer = Vec::new();
    let _size = reader.read_to_end(&mut buffer)?;
    Ok(buffer)
}

#[derive(Debug, Default)]
pub struct ImportConfig {
    /// For animated images; decides which frame will be used as static image.
    pub selected_frame: FrameIndex,
}

/// Zero-indexed frame index.
#[derive(Clone, Copy, Debug)]
pub enum FrameIndex {
    First,
    Last,
    Nth(usize),
}

impl Default for FrameIndex {
    fn default() -> Self {
        FrameIndex::First
    }
}

fn starts_with_gif_magic_number(buffer: &[u8]) -> bool {
    buffer.starts_with(b"GIF87a") || buffer.starts_with(b"GIF89a")
}

fn load_gif(buffer: &[u8], frame: FrameIndex) -> Result<image::DynamicImage, ImportError> {
    let decoder = image::gif::Decoder::new(&buffer[..])?;
    let frames = decoder.into_frames();
    let vec = frames.collect::<Result<Vec<_>, image::ImageError>>()?;
    let amount_of_frames = vec.len();

    // The one-indexed selected frame picked by the user; stored as zero-indexed frames
    // in the import config.
    // There is no guarantee that the selected frame does exist at this point.
    let selected = match frame {
        FrameIndex::First => 0usize,
        FrameIndex::Nth(n) => n,
        FrameIndex::Last => {
            if vec.is_empty() {
                return Err(ImportError::NoSuchFrame(0, "No frames found.".to_string()));
            }

            amount_of_frames - 1
        }
    };

    // Check that the frame exists, because we will access the buffer unchecked.
    if selected >= amount_of_frames {
        return Err(ImportError::NoSuchFrame(
            selected,
            format!(
                "Chosen frame index exceeds the maximum frame index ({}) of the image.",
                amount_of_frames
            ),
        ));
    }

    // select the frame from the buffer.
    let pick = &vec[selected];

    // fixme: Can we get away without cloning?
    let image = pick.clone().into_buffer();
    Ok(image::DynamicImage::ImageRgba8(image))
}

#[derive(Debug)]
pub enum ImportError {
    Image(image::ImageError),
    Io(std::io::Error),
    NoSuchFrame(usize, String),
}

impl From<std::io::Error> for ImportError {
    fn from(error: std::io::Error) -> Self {
        ImportError::Io(error)
    }
}

impl From<image::ImageError> for ImportError {
    fn from(error: image::ImageError) -> Self {
        ImportError::Image(error)
    }
}

#[deprecated(since = "0.2.0", note = "Errors as Strings to be phased out.")]
impl From<ImportError> for String {
    fn from(error: ImportError) -> String {
        match error {
            ImportError::Io(err) => err.description().to_string(),
            ImportError::Image(err) => err.to_string(),
            ImportError::NoSuchFrame(which, reason) => format!(
                "Unable to extract frame {} from the (animated) image. Reason given: {}",
                which + 1,
                reason,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;
    use sic_testing::*;

    const GIF_LOOP: &str = "loop.gif";
    const GIF_NO_LOOP: &str = "noloop.gif";
    const XY: u32 = 10;

    #[test]
    fn load_gif_non_looping_frame_first() {
        let load_path = setup_test_image(GIF_NO_LOOP);

        let config = ImportConfig {
            selected_frame: FrameIndex::First,
        };

        let image = load_image(&mut file_reader(load_path).unwrap(), &config).unwrap();

        // color = red
        let expected: [u8; 4] = [254, 0, 0, 255];
        assert_eq!(image.get_pixel(XY, XY).0, expected);
    }

    #[test]
    fn load_gif_non_looping_frame_first_is_zero() {
        let load_path = setup_test_image(GIF_NO_LOOP);

        let first = ImportConfig {
            selected_frame: FrameIndex::First,
        };

        let zero = ImportConfig {
            selected_frame: FrameIndex::Nth(0),
        };

        let first = load_image(&mut file_reader(&load_path).unwrap(), &first).unwrap();
        let zero = load_image(&mut file_reader(&load_path).unwrap(), &zero).unwrap();

        assert_eq!(first.get_pixel(XY, XY).0, zero.get_pixel(XY, XY).0);
    }

    #[test]
    fn load_gif_looping_frame_first_is_zero() {
        let load_path = setup_test_image(GIF_LOOP);

        let first = ImportConfig {
            selected_frame: FrameIndex::First,
        };

        let zero = ImportConfig {
            selected_frame: FrameIndex::Nth(0),
        };

        let first = load_image(&mut file_reader(&load_path).unwrap(), &first).unwrap();
        let zero = load_image(&mut file_reader(&load_path).unwrap(), &zero).unwrap();

        assert_eq!(first.get_pixel(XY, XY).0, zero.get_pixel(XY, XY).0);
    }

    // [[expected color]; amount]
    // verified with pastel cli;
    const FRAME_COLORS: [[u8; 4]; 8] = [
        [254, 0, 0, 255],     // red
        [254, 165, 0, 255],   // orange
        [255, 255, 0, 255],   // yellow
        [0, 128, 1, 255],     // green
        [0, 0, 254, 255],     // blue
        [75, 0, 129, 255],    // indigo
        [238, 130, 239, 255], // violet
        [0, 0, 0, 255],       // black
    ];

    #[test]
    fn load_gif_non_looping_frame_nth() {
        for (i, expected) in FRAME_COLORS.iter().enumerate() {
            let load_path = setup_test_image(GIF_NO_LOOP);

            let config = ImportConfig {
                selected_frame: FrameIndex::Nth(i),
            };

            let image = load_image(&mut file_reader(load_path).unwrap(), &config).unwrap();

            assert_eq!(&image.get_pixel(XY, XY).0, expected);
        }
    }

    #[test]
    fn load_gif_looping_frame_nth() {
        for (i, expected) in FRAME_COLORS.iter().enumerate() {
            let load_path = setup_test_image(GIF_LOOP);

            let config = ImportConfig {
                selected_frame: FrameIndex::Nth(i),
            };

            let image = load_image(&mut file_reader(load_path).unwrap(), &config).unwrap();

            assert_eq!(&image.get_pixel(XY, XY).0, expected);
        }
    }

    #[test]
    fn load_gif_non_looping_frame_nth_beyond_length() {
        let load_path = setup_test_image(GIF_NO_LOOP);

        let config = ImportConfig {
            selected_frame: FrameIndex::Nth(8),
        };

        let result = load_image(&mut file_reader(load_path).unwrap(), &config);
        assert!(result.is_err());
    }

    // Even if the gif loops, it still has 8 frames.
    #[test]
    fn load_gif_looping_frame_nth_beyond_length() {
        let load_path = setup_test_image(GIF_LOOP);

        let config = ImportConfig {
            selected_frame: FrameIndex::Nth(8),
        };

        let result = load_image(&mut file_reader(load_path).unwrap(), &config);
        assert!(result.is_err());
    }

    #[test]
    fn load_gif_non_looping_frame_last_is_seven_index() {
        let load_path = setup_test_image(GIF_NO_LOOP);

        let last = ImportConfig {
            selected_frame: FrameIndex::Last,
        };

        let seven = ImportConfig {
            selected_frame: FrameIndex::Nth(7),
        };

        let last = load_image(&mut file_reader(&load_path).unwrap(), &last).unwrap();
        let seven = load_image(&mut file_reader(&load_path).unwrap(), &seven).unwrap();

        assert_eq!(last.get_pixel(XY, XY).0, seven.get_pixel(XY, XY).0);
    }

    #[test]
    fn load_gif_looping_frame_last_is_seven_index() {
        let load_path = setup_test_image(GIF_LOOP);

        let last = ImportConfig {
            selected_frame: FrameIndex::Last,
        };

        let seven = ImportConfig {
            selected_frame: FrameIndex::Nth(7),
        };

        let last = load_image(&mut file_reader(&load_path).unwrap(), &last).unwrap();
        let seven = load_image(&mut file_reader(&load_path).unwrap(), &seven).unwrap();

        assert_eq!(last.get_pixel(XY, XY).0, seven.get_pixel(XY, XY).0);
    }

    const NOT_GIFS: [&str; 3] = [
        "blackwhite_2x2.bmp",
        "bwlines.png",
        "unsplash_763569_cropped.jpg",
    ];

    #[test]
    fn load_not_gif_formatted() {
        for path in NOT_GIFS.iter() {
            let load_path = setup_test_image(path);
            let config = ImportConfig::default();
            let result = load_image(&mut file_reader(load_path).unwrap(), &config);
            assert!(result.is_ok());
        }
    }
}
