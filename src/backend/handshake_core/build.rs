fn main() {
    // Ensure sqlx uses the prepared offline data by default; allow override via env.
    if std::env::var("SQLX_OFFLINE").is_err() {
        println!("cargo:rustc-env=SQLX_OFFLINE=true");
    }
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=src");
}
