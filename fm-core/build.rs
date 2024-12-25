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
        .flag("/Ob3")
        // .flag("/favor:AMD64")
        .flag("/GS-")
        .flag("/Gv")
        // .flag("/GL")
        .flag("/utf-8")
        // .flag("/TC")
        .flag("/fp:except-")
        .flag("/vlen=256")
        .flag("/Zc:tlsGuards-")
        .include("cfiles")
        .compile("freq_modulation");
}
