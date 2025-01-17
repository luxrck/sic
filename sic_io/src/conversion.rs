use std::io::{self, Write};
use std::path::Path;

use sic_core::image;

use crate::save::ExportMethod;

#[derive(Clone, Copy, Debug)]
pub enum AutomaticColorTypeAdjustment {
    // Usually the default
    Enabled,
    Disabled,
}

impl Default for AutomaticColorTypeAdjustment {
    fn default() -> Self {
        AutomaticColorTypeAdjustment::Enabled
    }
}

/// Use the ConversionWriter to convert and write image buffers to an output.
pub struct ConversionWriter<'a> {
    image: &'a image::DynamicImage,
}

impl<'a> ConversionWriter<'a> {
    pub fn new(image: &image::DynamicImage) -> ConversionWriter {
        ConversionWriter { image }
    }

    pub fn write<P: AsRef<Path>>(
        &self,
        export: ExportMethod<P>,
        output_format: image::ImageOutputFormat,
        color_type_adjustment: AutomaticColorTypeAdjustment,
    ) -> Result<(), String> {
        let color_processing = &ConversionWriter::pre_process_color_type(
            &self.image,
            &output_format,
            color_type_adjustment,
        );

        let export_buffer = match color_processing {
            Some(replacement) => replacement,
            None => &self.image,
        };

        match export {
            // Some() => write to file
            ExportMethod::File(v) => {
                ConversionWriter::save_to_file(&export_buffer, output_format, v)
            }
            // None => write to stdout
            ExportMethod::StdoutBytes => {
                ConversionWriter::export_to_stdout(&export_buffer, output_format)
            }
        }
    }

    /// Some image output format types require color type pre-processing.
    /// This is the case if the output image format does not support the color type held by the image buffer prior to the final conversion.
    ///
    /// If pre-processing of the color type took place, Some(<new image>) will be returned.
    /// If no pre-processing of the color type is required will return None.
    fn pre_process_color_type(
        image: &image::DynamicImage,
        output_format: &image::ImageOutputFormat,
        color_type_adjustment: AutomaticColorTypeAdjustment,
    ) -> Option<image::DynamicImage> {
        // A remaining open question: does a user expect for an image to be able to convert to a format even if the color type is not supported?
        // And even if the user does, should we?
        // I suspect that users expect that color type conversions should happen automatically.
        //
        // Testing also showed that even bmp with full black full white pixels do not convert correctly as of now. Why exactly is unclear;
        // Perhaps the color type of the bmp formatted test image?

        match color_type_adjustment {
            AutomaticColorTypeAdjustment::Enabled => match output_format {
                image::ImageOutputFormat::PNM(image::pnm::PNMSubtype::Bitmap(_)) => {
                    Some(image.grayscale())
                }
                image::ImageOutputFormat::PNM(image::pnm::PNMSubtype::Graymap(_)) => {
                    Some(image.grayscale())
                }
                image::ImageOutputFormat::PNM(image::pnm::PNMSubtype::Pixmap(_)) => {
                    Some(image::DynamicImage::ImageRgb8(image.to_rgb()))
                }
                _ => None,
            },
            AutomaticColorTypeAdjustment::Disabled => None,
        }
    }

    fn save_to_file<P: AsRef<Path>>(
        buffer: &image::DynamicImage,
        format: image::ImageOutputFormat,
        path: P,
    ) -> Result<(), String> {
        let mut out = std::fs::File::create(path).map_err(|err| err.to_string())?;

        buffer
            .write_to(&mut out, format)
            .map_err(|err| err.to_string())
    }

    fn export_to_stdout(
        buffer: &image::DynamicImage,
        format: image::ImageOutputFormat,
    ) -> Result<(), String> {
        let mut write_buffer = Vec::new();

        buffer
            .write_to(&mut write_buffer, format)
            .map_err(|err| err.to_string())?;

        io::stdout()
            .write(&write_buffer)
            .map(|_| ())
            .map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use sic_testing::{clean_up_output_path, setup_output_path, setup_test_image};

    use super::*;

    // Individual tests:

    const INPUT: &str = "rainbow_8x6.bmp";
    const OUTPUT: &str = "_out.png";

    #[test]
    fn will_output_file_be_created() {
        let our_output = &format!("will_output_file_be_created{}", OUTPUT); // this is required because tests are run in parallel, and the creation, or deletion can collide.
        let output_path = setup_output_path(our_output);

        let buffer = image::open(setup_test_image(INPUT)).expect("Can't open test file.");
        let example_output_format = image::ImageOutputFormat::PNG;
        let conversion_processor = ConversionWriter::new(&buffer);
        conversion_processor
            .write(
                ExportMethod::File(&output_path),
                example_output_format,
                AutomaticColorTypeAdjustment::Enabled,
            )
            .expect("Unable to save file to the test computer.");

        assert!(output_path.exists());

        clean_up_output_path(our_output);
    }

    #[test]
    fn has_png_extension() {
        let our_output = &format!("has_png_extension{}", OUTPUT); // this is required because tests are run in parallel, and the creation, or deletion can collide.
        let output_path = setup_output_path(our_output);

        let buffer = image::open(setup_test_image(INPUT)).expect("Can't open test file.");
        let example_output_format = image::ImageOutputFormat::PNG;
        let conversion_processor = ConversionWriter::new(&buffer);
        conversion_processor
            .write(
                ExportMethod::File(&output_path),
                example_output_format,
                AutomaticColorTypeAdjustment::Enabled,
            )
            .expect("Unable to save file to the test computer.");

        assert_eq!(
            Some(std::ffi::OsStr::new("png")),
            setup_output_path(our_output).extension()
        );

        clean_up_output_path(our_output);
    }

    #[test]
    fn is_png_file() {
        let our_output = &format!("is_png_file{}", OUTPUT); // this is required because tests are run in parallel, and the creation, or deletion can collide.
        let output_path = setup_output_path(our_output);

        let buffer = image::open(setup_test_image(INPUT)).expect("Can't open test file.");
        let example_output_format = image::ImageOutputFormat::PNG;
        let conversion_processor = ConversionWriter::new(&buffer);
        conversion_processor
            .write(
                ExportMethod::File(&output_path),
                example_output_format,
                AutomaticColorTypeAdjustment::Enabled,
            )
            .expect("Unable to save file to the test computer.");

        let mut file = std::fs::File::open(setup_output_path(our_output))
            .expect("Unable to find file we made.");
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)
            .expect("Unable to finish reading our test image.");

        assert_eq!(
            image::ImageFormat::PNG,
            image::guess_format(&bytes).expect("Format could not be guessed.")
        );

        clean_up_output_path(our_output);
    }

    // Multi tests:
    // Below all supported formats are testsed using the inputs listed below.

    const INPUT_MULTI: &[&str] = &["blackwhite_2x2.bmp", "palette_4x4.png"];
    const INPUT_FORMATS: &[&str] = &[
        "bmp", "gif", "ico", "jpg", "jpeg", "png", "pbm", "pgm", "ppm", "pam",
    ];
    const OUTPUT_FORMATS: &[image::ImageOutputFormat] = &[
        image::ImageOutputFormat::BMP,
        image::ImageOutputFormat::GIF,
        image::ImageOutputFormat::ICO,
        image::ImageOutputFormat::JPEG(80),
        image::ImageOutputFormat::JPEG(80),
        image::ImageOutputFormat::PNG,
        image::ImageOutputFormat::PNM(image::pnm::PNMSubtype::Bitmap(
            image::pnm::SampleEncoding::Binary,
        )),
        image::ImageOutputFormat::PNM(image::pnm::PNMSubtype::Graymap(
            image::pnm::SampleEncoding::Binary,
        )),
        image::ImageOutputFormat::PNM(image::pnm::PNMSubtype::Pixmap(
            image::pnm::SampleEncoding::Binary,
        )),
        image::ImageOutputFormat::PNM(image::pnm::PNMSubtype::ArbitraryMap),
    ];

    const EXPECTED_VALUES: &[image::ImageFormat] = &[
        image::ImageFormat::BMP,
        image::ImageFormat::GIF,
        image::ImageFormat::ICO,
        image::ImageFormat::JPEG,
        image::ImageFormat::JPEG,
        image::ImageFormat::PNG,
        image::ImageFormat::PNM,
        image::ImageFormat::PNM,
        image::ImageFormat::PNM,
        image::ImageFormat::PNM,
    ];

    fn test_conversion_with_header_match(
        input: &str,
        enc_format: &str,
        format: image::ImageOutputFormat,
        expected_format: image::ImageFormat,
    ) {
        let our_output = &format!("header_match_conversion.{}", enc_format); // this is required because tests are run in parallel, and the creation, or deletion can collide.
        let output_path = setup_output_path(our_output);

        let buffer = image::open(setup_test_image(input)).expect("Can't open test file.");
        let conversion_processor = ConversionWriter::new(&buffer);
        let method = ExportMethod::File(&output_path);

        conversion_processor
            .write(method, format, AutomaticColorTypeAdjustment::Enabled)
            .expect("Unable to save file to the test computer.");

        let mut file = std::fs::File::open(setup_output_path(our_output))
            .expect("Unable to find file we made.");
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)
            .expect("Unable to finish reading our test image.");

        assert_eq!(
            expected_format,
            image::guess_format(&bytes).expect("Format could not be guessed.")
        );

        clean_up_output_path(our_output);
    }

    #[test]
    fn test_conversions_with_header_match() {
        for test_image in INPUT_MULTI.iter() {
            let zipped = INPUT_FORMATS
                .iter()
                .zip(OUTPUT_FORMATS.iter().cloned())
                .zip(EXPECTED_VALUES.iter());

            for ((ext, to_format), expected_format) in zipped {
                println!(
                    "testing `test_conversion_with_header_match`, converting {} => : {}",
                    test_image, ext
                );
                test_conversion_with_header_match(test_image, ext, to_format, *expected_format);
            }
        }
    }
}
