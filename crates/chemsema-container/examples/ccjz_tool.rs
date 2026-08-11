use chemsema_container::{decode_ccjz, encode_ccjz};
use std::fs;

fn main() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, input, output] if command == "encode" => {
            let text = fs::read_to_string(input)
                .map_err(|error| format!("Failed to read {input}: {error}"))?;
            let bytes = encode_ccjz(&text)?;
            fs::write(output, bytes)
                .map_err(|error| format!("Failed to write {output}: {error}"))?;
        }
        [command, input] if command == "decode" => {
            let bytes =
                fs::read(input).map_err(|error| format!("Failed to read {input}: {error}"))?;
            println!("{}", decode_ccjz(&bytes)?);
        }
        _ => {
            return Err(
                "Usage: ccjz_tool encode <input.ccjs> <output.ccjz> | decode <input.ccjz>"
                    .to_string(),
            )
        }
    }
    Ok(())
}
