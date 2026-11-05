use std::{env, fs, path::Path};

fn generate_catalogs() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let messages = Path::new(&manifest).join("../../../packages/ui/src/contract/messages.json");
    let locales = Path::new(&manifest).join("../locales");
    println!("cargo:rerun-if-changed={}", messages.display());
    println!("cargo:rerun-if-changed={}", locales.display());
    let messages: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&fs::read_to_string(messages).unwrap()).unwrap();
    let mut files: Vec<_> = fs::read_dir(locales)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    files.sort();
    let catalogs: Vec<_> = files
        .iter()
        .map(|path| {
            let tag = path.file_stem().unwrap().to_str().unwrap();
            format!("({tag:?}, include_str!({:?}))", path.display().to_string())
        })
        .collect();
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("catalogs.rs"),
        format!(
            "const KEYS: &[&str] = &{:?};\nconst CATALOGS: &[(&str, &str)] = &[{}];\n",
            messages.keys().collect::<Vec<_>>(),
            catalogs.join(", ")
        ),
    )
    .unwrap();
}

fn main() {
    generate_catalogs();
    tauri_build::build()
}
