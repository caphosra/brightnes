fn main() {
    println!("cargo:rerun-if-changed=./kernel/kernel.ld");
    println!("cargo:rustc-link-arg=-T./kernel/kernel.ld");
}
