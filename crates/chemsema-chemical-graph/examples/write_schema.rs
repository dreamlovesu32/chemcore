use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: write_schema <output-path>")?;
    fs::write(path, chemsema_chemical_graph::json_schema_pretty()?)?;
    Ok(())
}
