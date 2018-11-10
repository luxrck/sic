#![feature(range_contains)]

use std::path::Path;

use clap::{App, Arg};
use image;
#[macro_use]
extern crate pest_derive;

use crate::config::{
    Config, FormatEncodingSettings, HelpDisplayProcessor, ImageOperationsProcessor,
    JPEGEncodingSettings, LicenseDisplayProcessor, PNMEncodingSettings, ProcessMutWithConfig,
    ProcessWithConfig, SelectedLicenses,
};

mod config;
mod conversion;
mod help;
mod operations;

const HELP_OPERATIONS_AVAILABLE: &str = include_str!("../docs/cli_help_script.txt");

fn main() -> Result<(), String> {
    let matches = App::new("Simple Image Converter")
        .version("0.7.2")
        .author("Martijn Gribnau <garm@ilumeo.com>")
        .about("Converts an image from one format to another.\n\n\
                Supported input formats are described BMP, GIF, ICO, JPEG, PNG, PPM (limitations may apply). \n\n\
                The image conversion is actually done by the awesome 'image' crate [1]. \n\
                Sic itself is a small command line frontend which supports a small part of the \
                conversion operations supported by the 'image' library. \n\n\
                [1] image crate by PistonDevelopers: https://github.com/PistonDevelopers/image \n\n\
                ")
        .arg(Arg::with_name("forced_output_format")
            .short("f")
            .long("force-format")
            .value_name("FORMAT")
            .help("Output formats supported: JPEG, PNG, GIF, ICO, PPM")
            .takes_value(true))
        .arg(Arg::with_name("license")
            .long("license")
            .help("Displays the license of the `sic` software.")
            .takes_value(false))
        .arg(Arg::with_name("dep_licenses")
            .long("dep-licenses")
            .help("Displays the licenses of the dependencies on which this software relies.")
            .takes_value(false))
        .arg(Arg::with_name("user_manual")
            .long("user-manual")
            .short("H")
            .help("Displays help text for different topics such as each supported script operation. Run `sic -H index` to display a list of available topics.")
            .value_name("TOPIC")
            .takes_value(true))
        .arg(Arg::with_name("script")
            .long("script")
            .help(HELP_OPERATIONS_AVAILABLE)
            .value_name("SCRIPT")
            .takes_value(true))
        .arg(Arg::with_name("jpeg_encoding_quality")
            .long("jpeg-encoding-quality")
            .value_name("QUALITY")
            .takes_value(true))
        .arg(Arg::with_name("pnm_encoding_bitmap_ascii")
            .long("pnm-encoding-bitmap-ascii"))
        .arg(Arg::with_name("pnm_encoding_graymap_ascii")
            .long("pnm-encoding-graymap-ascii"))
        .arg(Arg::with_name("pnm_encoding_pixmap_ascii")
            .long("pnm-encoding-pixmap-ascii"))
        .arg(Arg::with_name("pnm_encoding_bitmap_binary")
            .long("pnm-encoding-bitmap-binary"))
        .arg(Arg::with_name("pnm_encoding_graymap_binary")
            .long("pnm-encoding-graymap-binary"))
        .arg(Arg::with_name("pnm_encoding_pixmap_binary")
            .long("pnm-encoding-pixmap-binary"))
        .arg(Arg::with_name("pnm_encoding_arbitrarymap")
            .long("pnm-encoding-arbitrarymap"))    
        .arg(Arg::with_name("input_file")
            .help("Sets the input file")
            .value_name("INPUT_FILE")
            .required_unless_one(&["license", "dep_licenses", "user_manual"])
            .index(1))
        .arg(Arg::with_name("output_file")
            .help("Sets the desired output file")
            .value_name("OUTPUT_FILE")
            .required_unless_one(&["license", "dep_licenses", "user_manual"])
            .index(2))
        .get_matches();

    // Here any option will panic when invalid.
    let options = Config {
        licenses: match (
            matches.is_present("license"),
            matches.is_present("dep_licenses"),
        ) {
            (true, true) => vec![
                SelectedLicenses::ThisSoftware,
                SelectedLicenses::Dependencies,
            ],
            (true, _) => vec![SelectedLicenses::ThisSoftware],
            (_, true) => vec![SelectedLicenses::Dependencies],
            _ => vec![],
        },

        user_manual: matches.value_of("user_manual").map(String::from),

        script: matches.value_of("script").map(String::from),

        forced_output_format: matches.value_of("forced_output_format").map(String::from),

        encoding_settings: FormatEncodingSettings {
            // 3 possibilities:
            //   - present + i (1 ... 100)
            //   - present + i !(1 ... 100)
            //   - not present (take default)
            jpeg_settings: JPEGEncodingSettings::new((
                matches.is_present("jpeg_encoding_quality"),
                matches.value_of("jpeg_encoding_quality"),
            )),
            pnm_settings: PNMEncodingSettings::new(&[
                matches.is_present("pnm_encoding_bitmap_ascii"),
                matches.is_present("pnm_encoding_graymap_ascii"),
                matches.is_present("pnm_encoding_pixmap_ascii"),
                matches.is_present("pnm_encoding_bitmap_binary"),
                matches.is_present("pnm_encoding_graymap_binary"),
                matches.is_present("pnm_encoding_pixmap_binary"),
                matches.is_present("pnm_encoding_arbitrarymap"),
            ]),
        },
    };

    let license_display_processor = LicenseDisplayProcessor::new();
    license_display_processor.process(&options);

    let help_display_processor = HelpDisplayProcessor::new();
    help_display_processor.process(&options);

    let input = matches
        .value_of("input_file")
        .ok_or_else(|| String::from("An INPUT was expected, but none was given."))
        .map(|input_str| Path::new(input_str));

    // open image, -> DynamicImage
    let mut buffer = input.and_then(|path| image::open(path).map_err(|err| err.to_string()))?;

    // perform image operations
    let mut image_operations_processor = ImageOperationsProcessor::new(&mut buffer);
    image_operations_processor.process_mut(&options)?;

    let output = matches
        .value_of("output_file")
        .ok_or_else(|| String::from("An OUTPUT was expected, but none was given."))?;

    match options.forced_output_format {
        Some(format) => conversion::convert_image_forced(&buffer, output, &format),
        None => conversion::convert_image_unforced(&buffer, output),
    }
}
