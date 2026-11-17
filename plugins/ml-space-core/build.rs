fn main() {
    let dir = "vendor/FFTConvolver";
    let mut b = cc::Build::new();
    b.cpp(true).std("c++14").include(dir).warnings(false);
    for name in [
        "AudioFFT.cpp",
        "Utilities.cpp",
        "FFTConvolver.cpp",
        "TwoStageFFTConvolver.cpp",
    ] {
        b.file(format!("{dir}/{name}"));
    }
    b.file("src/convolution.cpp").compile("ml_convolution");
    println!("cargo:rerun-if-changed=vendor/FFTConvolver");
    println!("cargo:rerun-if-changed=src/convolution.cpp");
}
