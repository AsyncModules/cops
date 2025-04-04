fn main() {
    let pwd = std::env::current_dir().unwrap();
    println!("cargo:rerun-if-changed=src/*");
    println!("cargo:rerun-if-changed=./link.lds");
    println!("cargo:rerun-if-changed=./build.rs");
    println!("cargo:rustc-cdylib-link-arg=-fuse-ld=lld");
    println!("cargo:rustc-link-arg=-T{}/link.lds", pwd.display());
    println!("cargo:rustc-link-arg=-soname=libcops.so");
    println!("cargo:rustc-link-arg=-fPIC");
}
