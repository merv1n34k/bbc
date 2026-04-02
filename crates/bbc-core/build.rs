use std::fs;
use std::path::Path;

fn main() {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let data_dir = data_dir.canonicalize().unwrap_or_else(|_| {
        panic!("data directory not found at {:?}", data_dir);
    });

    println!("cargo:rerun-if-changed={}", data_dir.display());

    let mut entries: Vec<(String, String)> = Vec::new();

    for entry in fs::read_dir(&data_dir).expect("cannot read data dir") {
        let entry = entry.expect("bad dir entry");
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "toml") {
            let stem = path.file_stem().unwrap().to_string_lossy().to_string();
            let content = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e));
            println!("cargo:rerun-if-changed={}", path.display());
            entries.push((stem, content));
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("unit_sets.rs");

    let mut code = String::new();
    code.push_str("/// Auto-generated from data/*.toml files\n");
    code.push_str("pub const UNIT_SETS: &[(&str, &str)] = &[\n");
    for (name, content) in &entries {
        code.push_str(&format!("    ({:?}, {:?}),\n", name, content));
    }
    code.push_str("];\n");

    fs::write(dest, code).expect("cannot write unit_sets.rs");
}
