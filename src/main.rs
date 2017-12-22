extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate pcx;
extern crate png;

error_chain! {}

// -------------------------------------------------------------------------------------------------

use clap::{Arg, App};

use png::HasParameters;

use std::io::{Read, Write, BufWriter};
use std::fs::{DirBuilder, File, OpenOptions, read_dir};
use std::path::{Path, PathBuf};

// -------------------------------------------------------------------------------------------------

const GFX_INPUT_DIR: &str = "GRAFIK";
const GFX_OUTPUT_DIR: &str = "GRAFIK_PNG";

const TEXT_INPUT_DIR: &str = "TEXT";
const TEXT_OUTPUT_DIR: &str = "TEXT_TXT";

#[cfg(windows)]
const LINE_ENDING: &str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &str = "\n";

// -------------------------------------------------------------------------------------------------

fn is_file_with_extension(path: &Path, extension_upper: &str) -> bool {
    if path.is_file() {
        path.extension()
            .map_or(false, |e| e.to_str().map_or(false, |e| e.to_uppercase() == extension_upper))
    } else {
        false
    }
}

fn create_output_file(path: &Path) -> ::Result<File> {
    OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)
        .chain_err(|| {
            format!("Unable to create '{}'. Is the path writable?",
                    path.to_string_lossy())
        })
}

fn to_output_filename(input_path: &Path,
                      output_path: &Path,
                      output_extension: &str) -> ::Result<PathBuf> {
    let stem = match input_path.file_stem() {
        Some(stem) => stem,
        None => bail!("Input path '{}' does not have a stem.", input_path.to_string_lossy()),
    };

    let mut output_filename = PathBuf::new();
    output_filename.push(output_path);
    output_filename.push(stem);
    output_filename.set_extension(output_extension);
    Ok(output_filename)
}

fn read_file_contents(filename: &Path) -> ::Result<Vec<u8>> {
    let mut input_file = OpenOptions::new().read(true).open(filename).chain_err(|| {
        format!("Unable to open input file '{}'.", filename.to_string_lossy())
    })?;
    let mut contents = Vec::new();
    let _ = input_file.read_to_end(&mut contents).chain_err(|| {
        format!("Unable to read input file '{}'.", filename.to_string_lossy())
    })?;
    Ok(contents)
}

fn format_err(err: &Error) -> String {
    let mut formatted_err = format!("error: {}\n", err);
    for e in err.iter().skip(1) {
        formatted_err.push_str(&format!("caused by: {}\n", e));
    }
    formatted_err
}

fn convert_pcx(input_filename: &Path, output_filename: &Path) -> ::Result<()> {
    let input_file_contents = {
        let mut input_file_contents = read_file_contents(input_filename)?;

        if input_file_contents.len() < 4 {
            bail!("'{}' is too small to be a valid PCX file.", input_filename.to_string_lossy());
        }

        input_file_contents[0] = 0x0a;
        input_file_contents[1] = 0x05;
        input_file_contents[2] = 0x01;
        input_file_contents[3] = 0x08;

        input_file_contents
    };

    let mut pcx_file = pcx::Reader::new(input_file_contents.as_slice()).chain_err(|| {
        format!("Unable to read contents of '{}' as PCX file.", input_filename.to_string_lossy())
    })?;

    if !pcx_file.is_paletted() || pcx_file.palette_length().unwrap_or(0) != 256 {
        bail!("'{}' does not contain a 256 color PCX palette.", input_filename.to_string_lossy());
    }

    let width = pcx_file.width() as usize;
    let height = pcx_file.height() as usize;

    let image_data = {
        let mut image_data = vec![0u8; width * height];
        for y in 0..height {
            let begin = width * y;
            let end = begin + width;
            pcx_file.next_row_paletted(&mut image_data[begin..end]).chain_err(|| {
                format!("Error occurred while decoding '{}'.", input_filename.to_string_lossy())
            })?;
        }
        image_data
    };

    let palette_data = {
        let mut palette_data = [0u8; 256 * 3];
        let _ = pcx_file.read_palette(&mut palette_data).chain_err(|| {
            format!("Error occurred while decoding palette of '{}'.",
                    input_filename.to_string_lossy())
        })?;
        palette_data
    };

    let writer = BufWriter::new(create_output_file(output_filename)?);
    let mut png_encoder = png::Encoder::new(writer, width as u32, height as u32);
    png_encoder.set(png::ColorType::Indexed).set(png::BitDepth::Eight);

    let mut png_writer = png_encoder.write_header().chain_err(|| {
        format!("Unable to write to '{}'.", input_filename.to_string_lossy())
    })?;

    png_writer.write_chunk(png::chunk::PLTE, &palette_data).chain_err(|| {
        format!("Unable to write to '{}'.", input_filename.to_string_lossy())
    })?;

    png_writer.write_image_data(&image_data).chain_err(|| {
        format!("Unable to write to '{}'.", input_filename.to_string_lossy())
    })?;

    Ok(())
}

fn convert_dir(input_path: &Path,
               input_extension: &str,
               output_path: &Path,
               output_extension: &str,
               conversion_fn: &Fn(&Path, &Path) -> ::Result<()>) -> ::Result<()> {
    let gfx_dir_reader = read_dir(&input_path).chain_err(|| {
        format!("Unable to read directory '{}'. Is the provided path correct?",
                input_path.to_string_lossy())
    })?;

    let _ = DirBuilder::new().create(&output_path).chain_err(|| {
        format!("Unable to create output directory '{}'. Is the path writable?",
                output_path.to_string_lossy())
    });

    for entry in gfx_dir_reader {
        let entry = entry.chain_err(|| {
            format!("Unable to read directory entry in '{}'.", input_path.to_string_lossy())
        })?;
        let input_filename = entry.path();
        if is_file_with_extension(&input_filename, input_extension) {
            let output_filename = to_output_filename(&input_filename,
                                                     &output_path,
                                                     output_extension).
                chain_err(|| {
                    format!("Unable to create output filename for input file '{}'.",
                            input_filename.to_string_lossy())
                })?;

            println!("Converting '{}' to '{}' ...",
                     input_filename.to_string_lossy(),
                     output_filename.to_string_lossy());

            let conversion_result = conversion_fn(&input_filename, &output_filename).chain_err(|| {
                format!("Unable to convert '{}' to '{}'.",
                        input_filename.to_string_lossy(),
                        output_filename.to_string_lossy())
            });

            if let Err(err) = conversion_result {
                println!("{}", format_err(&err));
            }
        }
    }

    Ok(())
}

fn convert_graphics(root_dir: &str) -> ::Result<()> {
    let gfx_input_path: PathBuf = [root_dir, GFX_INPUT_DIR].iter().collect();
    let gfx_output_path: PathBuf = [root_dir, GFX_OUTPUT_DIR].iter().collect();

    convert_dir(&gfx_input_path, "PCX", &gfx_output_path, "PNG", &convert_pcx)
}

fn convert_txt(input_filename: &Path, output_filename: &Path) -> ::Result<()> {
    let file_contents = read_file_contents(input_filename)?;

    let converted_file_contents = {
        let mut s = String::with_capacity(file_contents.len());
        s.push('\u{feff}');
        for &c in &file_contents {
            match c {
                10 => s.push_str(LINE_ENDING),
                11 ... 137 => s.push((c - 10) as char),
                139 => s.push('ü'),
                164 => s.push('Ü'),
                142 => s.push('ä'),
                152 => s.push('Ä'),
                158 => s.push('ö'),
                163 => s.push('Ö'),
                183 => s.push('ô'),
                235 => s.push('ß'),
                _ => bail!("'{}' contains illegal character {}",
                           input_filename.to_string_lossy(),
                           c),
            };
        }

        s
    };

    let mut file = create_output_file(output_filename)?;
    file.write_all(converted_file_contents.as_bytes()).chain_err(|| {
        format!("Unable to write to '{}'.", output_filename.to_string_lossy())
    })
}

fn convert_texts(root_dir: &str) -> ::Result<()> {
    let txt_input_path: PathBuf = [root_dir, TEXT_INPUT_DIR].iter().collect();
    let txt_output_path: PathBuf = [root_dir, TEXT_OUTPUT_DIR].iter().collect();

    convert_dir(&txt_input_path, "TCT", &txt_output_path, "TXT", &convert_txt)
}

fn pause() {
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    write!(stdout, "Press return to continue...").unwrap();
    stdout.flush().unwrap();

    let _ = stdin.read(&mut [0u8]).unwrap();
}

fn run(root_dir: &str) -> ::Result<()> {
    println!("Converting graphics ...");
    convert_graphics(root_dir)?;

    println!();

    println!("Converting texts ...");
    convert_texts(root_dir)?;

    Ok(())
}

fn main() {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::with_name("DIRECTORY")
            .help("The root directory of the 'Jonathan' game. By default the current directory is used.")
            .index(1))
        .after_help("The PCX files in the GRAFIK directory are converted to PNG files and written to the new directory GRAFIK_PNG.\n\
                     The TCT files in the TEXT directory are converted to UTF-8 text files and written to the new directory TEXT_TXT.")
        .get_matches();

    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!(env!("CARGO_PKG_AUTHORS"));
    println!();

    if let Err(ref err) = run(matches.value_of("DIRECTORY").unwrap_or(".")) {
        let formatted_err = format_err(err);

        use std::io::Write;
        let stderr = &mut std::io::stderr();
        let errmsg = "Error writing to stderr";
        writeln!(stderr, "{}", formatted_err).expect(errmsg);
        pause();

        std::process::exit(1);
    } else {
        pause();
    }
}
