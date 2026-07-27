use std::{env, fs, process};

fn main() {
    let mut args = env::args_os().skip(1);
    let Some(input) = args.next() else {
        eprintln!("usage: cdx_to_cdxml <input.cdx|input.ctp> <output.cdxml>");
        process::exit(2);
    };
    let Some(output) = args.next() else {
        eprintln!("usage: cdx_to_cdxml <input.cdx|input.ctp> <output.cdxml>");
        process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("usage: cdx_to_cdxml <input.cdx|input.ctp> <output.cdxml>");
        process::exit(2);
    }
    let bytes = fs::read(&input).unwrap_or_else(|error| {
        eprintln!("failed to read {}: {error}", input.to_string_lossy());
        process::exit(1);
    });
    let cdxml = chemsema_engine::cdx_to_cdxml(&bytes).unwrap_or_else(|error| {
        eprintln!("failed to decode {}: {error}", input.to_string_lossy());
        process::exit(1);
    });
    fs::write(&output, cdxml).unwrap_or_else(|error| {
        eprintln!("failed to write {}: {error}", output.to_string_lossy());
        process::exit(1);
    });
}
