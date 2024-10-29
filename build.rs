fn main() {
    let fname = "freq_modulation.c";
    let fpath1 = format!("cfiles/{}", fname);
    // let fpath2 = format!("cfiles/{}", "resampler.c");
    println!("{}", &format!("cargo:rerun-if-changed={}", fpath1));
    // println!("{}", &format!("cargo:rerun-if-changed={}", ));
    cc::Build::new()
        .file(fpath1)
        .flag("/arch:AVX2")
        .include("cfiles")
        .compile("freq_modulation");
    // cc::Build::new()
    //   .file(fpath2)
    //   .flag("/arch:AVX2")
    //   .include("cfiles")
    //   .compile("resampler");
}
