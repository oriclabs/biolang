fn main() {
    println!("cargo:rerun-if-changed=src/annoy_bridge.cpp");
    println!("cargo:rerun-if-changed=vendor/annoy/annoylib.h");
    println!("cargo:rerun-if-changed=vendor/annoy/kissrandom.h");
    cc::Build::new()
        .cpp(true)
        .file("src/annoy_bridge.cpp")
        .include("vendor/annoy")
        .flag_if_supported("-std=c++11")
        .compile("bl_seurat_annoy");
}
