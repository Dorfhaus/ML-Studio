fn main() {
    let root = "vendor/nam";
    let mut b = cc::Build::new();
    b.cpp(true)
        .std("c++20")
        .warnings(false)
        .cargo_metadata(false)
        .include(format!("{root}/NAM"))
        .include(format!("{root}/Dependencies/eigen"))
        .include(format!("{root}/Dependencies/nlohmann"))
        .include(root)
        .define("NAM_SAMPLE_FLOAT", None)
        .define("NOMINMAX", None)
        .define("WIN32_LEAN_AND_MEAN", None)
        .define("EIGEN_MPL2_ONLY", None)
        .flag_if_supported("/EHsc")
        .flag_if_supported("/bigobj");
    for folder in [format!("{root}/NAM"), format!("{root}/NAM/wavenet")] {
        for entry in std::fs::read_dir(folder).unwrap() {
            let p = entry.unwrap().path();
            if p.extension().is_some_and(|s| s == "cpp") {
                b.file(p);
            }
        }
    }
    b.file("src/nam.cpp").compile("ml_nam");
    println!(
        "cargo:rustc-link-search=native={}",
        std::env::var("OUT_DIR").unwrap()
    );
    // Every model translation unit registers its factory through a static constructor.
    println!("cargo:rustc-link-lib=static:+whole-archive=ml_nam");
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target == "macos" {
        println!("cargo:rustc-link-lib=c++");
    } else if target != "windows" {
        println!("cargo:rustc-link-lib=stdc++");
    }
    println!("cargo:rerun-if-changed=vendor/nam");
    println!("cargo:rerun-if-changed=src/nam.cpp");
}
