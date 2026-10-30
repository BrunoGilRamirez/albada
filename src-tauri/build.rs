use std::{env, fs, path::PathBuf};

fn main() {
    tauri_build::build();

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let migrations_dir = manifest_dir.join("migrations");
    println!("cargo:rerun-if-changed={}", migrations_dir.display());

    let mut migrations = fs::read_dir(&migrations_dir)
        .expect("migrations directory must exist while compiling")
        .map(|entry| {
            entry
                .expect("migration directory entry must be readable")
                .path()
        })
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("sql"))
        .collect::<Vec<_>>();
    migrations.sort();

    let mut generated = String::from("pub static MIGRATIONS: &[(&str, &str)] = &[\n");
    for path in migrations {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("migration file name must be valid UTF-8");
        generated.push_str(&format!(
            "    ({name:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/migrations/{name}\"))),\n"
        ));
    }
    generated.push_str("];\n");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out_dir.join("migration_sources.rs"), generated)
        .expect("generated migration source file must be writable");
}
