fn main() {
    // The embedded Surreal schema and its Rust bootstrap are the only storage build inputs.
    println!("cargo:rerun-if-changed=src");
}
