fn main() {
    let fname = "freq_modulation.c";
    let fpath1 = format!("cfiles/{}", fname);
    // let fpath2 = format!("cfiles/{}", "resampler.c");
    println!("{}", &format!("cargo:rerun-if-changed={}", fpath1));
    // println!("{}", &format!("cargo:rerun-if-changed={}", ));
    cc::Build::new()
        .file(fpath1)
        .flag("/arch:AVX2")
        .flag("/fp:fast")
        .flag("/favor:AMD64")
        .flag("/GA")
        .flag("/utf-8")
        // .flag("/TC")
        // .flag("/fp:precise")
        .include("cfiles")
        .compile("freq_modulation");
}
