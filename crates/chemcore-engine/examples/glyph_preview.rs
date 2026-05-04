use chemcore_engine::render_glyph_preview_svg;
use std::env;
use std::fs;

fn default_patterns() -> Vec<&'static str> {
    vec![
        "O",
        "Ca",
        "Ca@1",
        "Br",
        "Ph",
        "SO_2#right",
        "O_2S@2#left",
        "SO_2#above",
        "SO_2#below",
        "NH#above",
        "NH#below",
        "NTs#above",
        "NTs#below",
        "N^3",
        "Mg^2+",
        "SO_4^2-",
        "t-Bu",
        "HN",
        "CN",
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output_path = String::from("tmp/chemcore_glyph_preview.svg");
    let mut patterns: Vec<String> = default_patterns().into_iter().map(str::to_string).collect();
    let mut only_mode = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--only" {
            patterns.clear();
            only_mode = true;
            continue;
        }
        if arg == "-o" {
            if let Some(value) = args.next() {
                output_path = value;
            }
            continue;
        }
        if only_mode {
            patterns.push(arg);
            continue;
        }
        patterns.push(arg);
    }

    let pattern_refs: Vec<&str> = patterns.iter().map(String::as_str).collect();
    let svg = render_glyph_preview_svg(&pattern_refs);
    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&output_path, svg)?;
    println!("wrote {}", output_path);
    Ok(())
}
