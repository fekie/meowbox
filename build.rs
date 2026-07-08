use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
};

const FIRST_CRY_ID: u16 = 387;
// The linker provides a 4 MiB flash window shared by code and static
// data. Leave enough room for the firmware while maximizing the
// number of cries.
const MAX_EMBEDDED_CRY_BYTES: u64 = 3_500 * 1_024;

fn main() {
    linker_be_nice();
    generate_cries();
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    // make sure linkall.x is the last linker script (otherwise might
    // cause problems with flip-link)
    println!("cargo:rustc-link-arg=-Tlinkall.x");
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
