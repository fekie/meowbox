use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};

use image::{AnimationDecoder, codecs::gif::GifDecoder};

const FIRST_POKEMON_ID: u16 = 494;
const LAST_POKEMON_ID: u16 = 503;

fn main() {
    linker_be_nice();
    generate_cries();
    generate_pokemon_sprites();
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    // make sure linkall.x is the last linker script (otherwise might
    // cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}

fn generate_pokemon_sprites() {
    let output_dir =
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let mut metadata = BufWriter::new(
        File::create(output_dir.join("pokemon_sprites.rs"))
            .expect("failed to create Pokemon sprite metadata"),
    );
    writeln!(metadata, "&[").unwrap();

    for pokemon_id in FIRST_POKEMON_ID..=LAST_POKEMON_ID {
        let asset_path =
            format!("assets/pokemon-sprites/{pokemon_id}.gif");
        println!("cargo:rerun-if-changed={asset_path}");

        let decoder = GifDecoder::new(BufReader::new(
            File::open(&asset_path).unwrap_or_else(|error| {
                panic!("failed to open {asset_path}: {error}")
            }),
        ))
        .unwrap_or_else(|error| {
            panic!("failed to decode {asset_path}: {error}")
        });
        let frames = decoder
            .into_frames()
            .collect_frames()
            .unwrap_or_else(|error| {
                panic!("failed to decode frames from {asset_path}: {error}")
            });
        assert!(!frames.is_empty(), "{asset_path} has no frames");

        let width = frames[0].buffer().width();
        let height = frames[0].buffer().height();
        assert!(
            width * height < 1 << 15,
            "{asset_path} is too large for the sprite delta format"
        );
        let delta_filename = format!("pokemon_{pokemon_id}.delta565");
        let mut deltas = BufWriter::new(
            File::create(output_dir.join(&delta_filename))
                .expect("failed to create Pokemon sprite delta data"),
        );
        let (delays_ms, offsets) = write_sprite_deltas(
            &asset_path,
            frames,
            width,
            height,
            &mut deltas,
        );
        deltas
            .flush()
            .expect("failed to flush Pokemon sprite delta data");

        writeln!(
            metadata,
            "PokemonSprite {{ pokemon_id: {pokemon_id}, width: {width}, height: {height}, delays_ms: &{delays_ms:?}, offsets: &{offsets:?}, deltas: include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{delta_filename}\")) }},"
        )
        .unwrap();
    }

    writeln!(metadata, "]").unwrap();
}

fn write_sprite_deltas(
    asset_path: &str,
    frames: Vec<image::Frame>,
    width: u32,
    height: u32,
    deltas: &mut impl Write,
) -> (Vec<u64>, Vec<u32>) {
    let mut delays_ms = Vec::with_capacity(frames.len());
    let mut offsets = Vec::with_capacity(frames.len() + 1);
    let mut previous = vec![0_u32; (width * height) as usize];
    let mut bytes_written = 0_u32;

    for frame in frames {
        assert_eq!(frame.buffer().width(), width, "{asset_path}");
        assert_eq!(frame.buffer().height(), height, "{asset_path}");

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
            let transparent = alpha < 128;
            let color = if transparent {
                0_u16
            } else {
                ((red as u16 >> 3) << 11)
                    | ((green as u16 >> 2) << 5)
                    | (blue as u16 >> 3)
            };
            let state = color as u32 | ((transparent as u32) << 16);
            if !first_frame && state == previous[index] {
                continue;
            }

            previous[index] = state;
            let encoded_index =
                index as u16 | ((transparent as u16) << 15);
            deltas
                .write_all(&encoded_index.to_le_bytes())
                .expect("failed to write Pokemon sprite delta index");
            deltas
                .write_all(&color.to_le_bytes())
                .expect("failed to write Pokemon sprite delta color");
            bytes_written += 4;
        }
    }
    offsets.push(bytes_written);

    (delays_ms, offsets)
}

fn generate_cries() {
    println!("cargo:rerun-if-changed=sounds/cries");

    let mut cries = fs::read_dir("sounds/cries")
        .expect("failed to read sounds/cries")
        .map(|entry| {
            let entry =
                entry.expect("failed to read cry directory entry");
            let filename = entry
                .file_name()
                .into_string()
                .expect("cry filenames must be valid UTF-8");
            filename
        })
        .filter_map(|filename| {
            if !filename.ends_with(".pcm") {
                return None;
            }

            let id = filename
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .collect::<String>()
                .parse::<u16>()
                .ok()?;

            (FIRST_POKEMON_ID..=LAST_POKEMON_ID)
                .contains(&id)
                .then_some((id, filename))
        })
        .collect::<Vec<_>>();

    cries.sort_by_key(|(id, _)| *id);
    assert_eq!(
        cries.len(),
        (LAST_POKEMON_ID - FIRST_POKEMON_ID + 1) as usize,
        "expected exactly one cry for each Pokemon from {FIRST_POKEMON_ID} through {LAST_POKEMON_ID}"
    );
    for (offset, (id, _)) in cries.iter().enumerate() {
        assert_eq!(*id, FIRST_POKEMON_ID + offset as u16);
    }

    let output_path =
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("cries.rs");
    let mut output = BufWriter::new(
        File::create(output_path).expect("failed to create cries.rs"),
    );

    writeln!(output, "&[").unwrap();
    for (pokemon_id, filename) in cries {
        writeln!(
            output,
            "Cry {{ pokemon_id: {pokemon_id}, filename: {filename:?}, samples: include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/sounds/cries/\", {filename:?})) }},"
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
