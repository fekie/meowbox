use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};

use image::{AnimationDecoder, codecs::gif::GifDecoder};

const FIRST_CRY_ID: u16 = 387;
// The linker provides a 4 MiB flash window shared by code and static
// data. Leave enough room for the firmware while maximizing the
// number of cries.
const MAX_EMBEDDED_CRY_BYTES: u64 = 3_500 * 1_024;

fn main() {
    linker_be_nice();
    generate_cries();
    generate_victini_animation();
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    // make sure linkall.x is the last linker script (otherwise might
    // cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}

fn generate_victini_animation() {
    const ASSET_PATH: &str = "assets/victini.gif";

    println!("cargo:rerun-if-changed={ASSET_PATH}");

    let decoder = GifDecoder::new(BufReader::new(
        File::open(ASSET_PATH).expect("failed to open Victini GIF"),
    ))
    .expect("failed to decode Victini GIF");
    let frames = decoder
        .into_frames()
        .collect_frames()
        .expect("failed to decode Victini GIF frames");
    assert!(!frames.is_empty(), "Victini GIF has no frames");

    let width = frames[0].buffer().width();
    let height = frames[0].buffer().height();
    let output_dir =
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let mut deltas = BufWriter::new(
        File::create(output_dir.join("victini.delta565"))
            .expect("failed to create Victini delta data"),
    );
    let mut delays_ms = Vec::with_capacity(frames.len());
    let mut offsets = Vec::with_capacity(frames.len() + 1);
    let mut previous = vec![0; (width * height) as usize];
    let mut bytes_written = 0_u32;

    for frame in frames {
        assert_eq!(frame.buffer().width(), width);
        assert_eq!(frame.buffer().height(), height);

        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let delay_ms = ((numerator as u64 + denominator as u64 - 1)
            / denominator as u64)
            .max(10);
        delays_ms.push(delay_ms);

        let first_frame = offsets.is_empty();
        offsets.push(bytes_written);
        for (index, pixel) in frame.into_buffer().pixels().enumerate()
        {
            let [red, green, blue, alpha] = pixel.0;
            let color = if alpha < 128 {
                0
            } else {
                ((red as u16 >> 3) << 11)
                    | ((green as u16 >> 2) << 5)
                    | (blue as u16 >> 3)
            };
            if !first_frame && color == previous[index] {
                continue;
            }

            previous[index] = color;
            deltas
                .write_all(&(index as u16).to_le_bytes())
                .expect("failed to write Victini delta index");
            deltas
                .write_all(&color.to_le_bytes())
                .expect("failed to write Victini delta color");
            bytes_written += 4;
        }
    }
    offsets.push(bytes_written);
    deltas.flush().expect("failed to flush Victini delta data");

    let mut metadata = BufWriter::new(
        File::create(output_dir.join("victini.rs"))
            .expect("failed to create Victini metadata"),
    );
    writeln!(metadata, "pub const VICTINI_WIDTH: u32 = {width};")
        .unwrap();
    writeln!(metadata, "pub const VICTINI_HEIGHT: u32 = {height};")
        .unwrap();
    writeln!(
        metadata,
        "pub const VICTINI_DELAYS_MS: &[u64] = &{delays_ms:?};"
    )
    .unwrap();
    writeln!(
        metadata,
        "pub const VICTINI_OFFSETS: &[u32] = &{offsets:?};"
    )
    .unwrap();
    writeln!(
        metadata,
        "pub static VICTINI_DELTAS: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/victini.delta565\"));"
    )
    .unwrap();
}

// currently generates around ~40 random cries for the pool
fn generate_cries() {
    println!("cargo:rerun-if-changed=sounds/cries");

    let mut cries = fs::read_dir("sounds/cries")
        .expect("failed to read sounds/cries")
        .map(|entry| {
            let entry =
                entry.expect("failed to read cry directory entry");
            let size = entry
                .metadata()
                .expect("failed to read cry metadata")
                .len();
            let filename = entry
                .file_name()
                .into_string()
                .expect("cry filenames must be valid UTF-8");
            (filename, size)
        })
        .filter_map(|(filename, size)| {
            if !filename.ends_with(".pcm") {
                return None;
            }

            let id = filename
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect::<String>()
                .parse::<u16>()
                .ok()?;

            (id >= FIRST_CRY_ID).then_some((id, filename, size))
        })
        .collect::<Vec<_>>();

    cries.sort_by(
        |(left_id, left_name, left_size),
         (right_id, right_name, right_size)| {
            left_size
                .cmp(right_size)
                .then_with(|| left_id.cmp(right_id))
                .then_with(|| left_name.cmp(right_name))
        },
    );

    let mut embedded_bytes = 0;
    cries.retain(|(_, _, size)| {
        if embedded_bytes + *size > MAX_EMBEDDED_CRY_BYTES {
            return false;
        }

        embedded_bytes += *size;
        true
    });
    cries.sort_by(
        |(left_id, left_name, _), (right_id, right_name, _)| {
            left_id
                .cmp(right_id)
                .then_with(|| left_name.cmp(right_name))
        },
    );
    assert!(!cries.is_empty(), "no Pokemon cries matched the filter");

    let output_path =
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("cries.rs");
    let mut output = BufWriter::new(
        File::create(output_path).expect("failed to create cries.rs"),
    );

    writeln!(output, "&[").unwrap();
    for (_, filename, _) in cries {
        writeln!(
            output,
            "Cry {{ filename: {filename:?}, samples: include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/sounds/cries/\", {filename:?})) }},"
        )
        .unwrap();
    }
    writeln!(output, "]").unwrap();
}

fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                what if what.starts_with("_defmt_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `defmt` not found - make sure `defmt.x` is added as a linker script and you have included `use defmt_rtt as _;`"
                    );
                    eprintln!();
                }
                "_stack_start" => {
                    eprintln!();
                    eprintln!(
                        "💡 Is the linker script `linkall.x` missing?"
                    );
                    eprintln!();
                }
                what if what.starts_with("esp_rtos_") => {
                    eprintln!();
                    eprintln!(
                        "💡 `esp-radio` has no scheduler enabled. Make sure you have initialized `esp-rtos` or provided an external scheduler."
                    );
                    eprintln!();
                }
                "embedded_test_linker_file_not_added_to_rustflags" => {
                    eprintln!();
                    eprintln!(
                        "💡 `embedded-test` not found - make sure `embedded-test.x` is added as a linker script for tests"
                    );
                    eprintln!();
                }
                "free"
                | "malloc"
                | "calloc"
                | "get_free_internal_heap_size"
                | "malloc_internal"
                | "realloc_internal"
                | "calloc_internal"
                | "free_internal" => {
                    eprintln!();
                    eprintln!(
                        "💡 Did you forget the `esp-alloc` dependency or didn't enable the `compat` feature on it?"
                    );
                    eprintln!();
                }
                _ => (),
            },
            // we don't have anything helpful for "missing-lib" yet
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=-Wl,--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}
