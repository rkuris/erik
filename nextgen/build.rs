use embuild::espidf::sysenv;
use std::{env, fs, path::PathBuf};

fn main() {
    sysenv::output();

    // Ensure the custom partition table is available alongside the generated sdkconfig
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set by Cargo"));
    let partitions_src = PathBuf::from("partitions.csv");
    let partitions_dst = out_dir.join("partitions.csv");

    println!("cargo:rerun-if-changed=partitions.csv");

    fs::copy(&partitions_src, &partitions_dst).unwrap_or_else(|err| {
        panic!(
            "Failed to copy {} to {}: {err}",
            partitions_src.display(),
            partitions_dst.display()
        )
    });
}
