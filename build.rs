fn main() {
    let fname = "freq_modulation.c";
    let fpath = format!("cfiles/{}", fname);
    println!("{}", &format!("cargo:rerun-if-changed={}", fpath));
    cc::Build::new()
        .file(fpath)
        .flag("/arch:AVX2")
        .include("cfiles")
        .compile("freq_modulation");
}
