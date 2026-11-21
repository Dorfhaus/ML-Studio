fn time_settings() {
    use std::fmt::Write;
    let mut entries = String::from("&[");
    for i in 0..22 { write!(entries,"LEGACY_TIME[{i}],").unwrap(); }
    let mut setting=|id:String,name:String,max:i32,start:i32| {
        writeln!(entries,"Setting {{id:{id:?},name:{name:?},range:(0,{max}),start:{start},magnet:None}},").unwrap();
    };
    for (id,name,max) in [("volume_slot","Volume slot",35),("time_slot","Time slot",35),("curve_shape","Time interpolation",2),("two_bars","Two-bar cycle",1)] {
        setting(id.into(),name.into(),max,0);
    }
    for (kind,first,initial) in [("time",0,0),("volume",1,1000)] {
        for slot in first..36 {for point in 0..16 {
            setting(format!("{kind}_{slot}_{point}"),format!("{} {} point {}",kind,slot+1,point+1),1000,initial);
        }}
    }
    entries.push(']');
    std::fs::write(std::path::Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("time_settings.rs"),entries).unwrap();
}
fn main() {
    time_settings();
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
