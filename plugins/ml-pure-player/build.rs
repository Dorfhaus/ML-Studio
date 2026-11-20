use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};
fn run(c: &mut Command) {
    let status = c
        .status()
        .expect("Run CMake (required to build the bundled FluidSynth engine)");
    assert!(status.success(), "FluidSynth build failed: {c:?}");
}
fn main() {
    // Do not canonicalize to Windows' extended \\?\ prefix: MSBuild treats it as a URL.
    let source =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("vendor/fluidsynth");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut build = out.join("fluidsynth");
    // Cargo can share OUT_DIR between worktrees. CMake's native cache belongs to
    // one source path; leave an active checkout's configured engine intact.
    if std::fs::read_to_string(build.join("CMakeCache.txt")).ok().and_then(|text| {
        text.lines().find_map(|line| line.strip_prefix("CMAKE_HOME_DIRECTORY:INTERNAL=").map(PathBuf::from))
    }).is_some_and(|home| home != source) {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut id = DefaultHasher::new();
        source.hash(&mut id);
        build = out.join(format!("fluidsynth-{:x}", id.finish()));
    }
    let target = env::var("TARGET").unwrap();
    let host = env::var("HOST").unwrap();
    let cache = build.join("CMakeCache.txt");
    if std::fs::read_to_string(&cache).is_ok_and(|text| text.contains("//?/")) {
        std::fs::remove_file(cache).expect("Discard obsolete generated CMake path cache");
    }
    let cmake = env::var_os("CMAKE").map(PathBuf::from).unwrap_or_else(|| {
        if cfg!(windows) {
            let vswhere =
                Path::new("C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe");
            if let Ok(o) = Command::new(vswhere)
                .args(["-latest", "-products", "*", "-find", "**/cmake.exe"])
                .output()
            {
                if o.status.success() {
                    if let Some(p) = String::from_utf8_lossy(&o.stdout).lines().next() {
                        return PathBuf::from(p.trim());
                    }
                }
            }
        }
        PathBuf::from("cmake")
    });
    std::fs::create_dir_all(&build).expect("Create native build directory");
    let mut configure = Command::new(&cmake);
    configure.current_dir(&build);
    configure
        .arg("-S")
        .arg(&source)
        .arg("-B")
        .arg(&build)
        .args([
            "-DCMAKE_BUILD_TYPE=Release",
            "-DBUILD_SHARED_LIBS=OFF",
            "-Denable-framework=OFF",
            "-Dosal=cpp11",
        ])
        .arg(format!(
            "-DGCEM_INCLUDE_DIR={}",
            source.join("gcem/include").display()
        ));
    for feature in [
        "alsa",
        "aufile",
        "dbus",
        "ipv6",
        "jack",
        "ladspa",
        "libsndfile",
        "midishare",
        "opensles",
        "oboe",
        "network",
        "oss",
        "dsound",
        "wasapi",
        "waveout",
        "winmidi",
        "sdl3",
        "pulseaudio",
        "pipewire",
        "readline",
        "openmp",
        "native-dls",
        "signalsmith",
        "systemd",
        "coreaudio",
        "coremidi",
    ] {
        configure.arg(format!("-Denable-{feature}=OFF"));
    }
    if host.contains("windows") && target.contains("windows-msvc") {
        configure.args([
            "-G",
            "Visual Studio 17 2022",
            "-A",
            if target.starts_with("aarch64") {
                "ARM64"
            } else {
                "x64"
            },
        ]);
    } else {
        // Visual Studio generators ignore the Linux cross compiler passed below.
        if host.contains("windows") && target.contains("linux") {
            configure.args(["-G", "Ninja"]);
            if let Some(root) = cmake.ancestors().nth(3) {
                let ninja = root.join("Ninja/ninja.exe");
                if ninja.is_file() {
                    configure.arg(format!("-DCMAKE_MAKE_PROGRAM={}", ninja.display()));
                }
            }
            if std::fs::read_to_string(build.join("CMakeCache.txt")).is_ok_and(|text|
                text.lines().any(|line| line.starts_with("CMAKE_GENERATOR:INTERNAL=") && !line.ends_with("=Ninja"))
            ) {
                configure.arg("--fresh");
            }
        }
        for (cpp, name) in [(false, "C"), (true, "CXX")] {
            let tool = cc::Build::new().cpp(cpp).get_compiler();
            configure.arg(format!("-DCMAKE_{name}_COMPILER={}", tool.path().display()));
            configure.arg(format!(
                "-DCMAKE_{name}_FLAGS_INIT={}",
                tool.args()
                    .iter()
                    .map(|a| a.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        if host != target {
            configure.arg("-DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY");
            configure.arg(format!(
                "-DCMAKE_SYSTEM_NAME={}",
                if target.contains("apple") {
                    "Darwin"
                } else {
                    "Linux"
                }
            ));
        }
        if target.contains("apple") {
            configure.arg(format!(
                "-DCMAKE_OSX_ARCHITECTURES={}",
                if target.starts_with("aarch64") {
                    "arm64"
                } else {
                    "x86_64"
                }
            ));
            configure.arg("-DCMAKE_OSX_DEPLOYMENT_TARGET=12.0");
        }
    }
    run(&mut configure);
    run(Command::new(cmake)
        .arg("--build")
        .arg(&build)
        .args([
            "--config",
            "Release",
            "--target",
            "libfluidsynth",
            "--parallel",
        ])
        .arg(env::var("NUM_JOBS").unwrap_or("4".into())));
    let windows = target.contains("windows");
    println!(
        "cargo:rustc-link-search=native={}",
        build
            .join(if windows { "src/Release" } else { "src" })
            .display()
    );
    println!(
        "cargo:rustc-link-lib=static={}",
        if windows {
            "libfluidsynth-3"
        } else {
            "fluidsynth"
        }
    );
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
    } else if !windows {
        for lib in ["stdc++", "m", "pthread"] {
            println!("cargo:rustc-link-lib={lib}");
        }
    }
    println!("cargo:rerun-if-changed=vendor/fluidsynth");
    println!("cargo:rerun-if-env-changed=CMAKE");
}
