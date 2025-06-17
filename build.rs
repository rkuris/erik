use embuild::espidf::sysenv;

use std::path::PathBuf;

fn main() {
    sysenv::output();
    let secretfile = PathBuf::from("src/secrets.rs");
    if !secretfile.exists() {
        println!("cargo::warning=Using secrets.rs.example, with some default secrets {}", std::env::current_dir().unwrap().display());
        std::fs::copy("src/secrets.rs.example", secretfile).expect("copy of secrets.rs.example to secrets.rs failed");
    }
}
