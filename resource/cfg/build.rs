fn main() {
    println!("cargo:rustc-env=RUSTFLAGS=-A non_snake_case");
    println!("cargo:rustc-env=RUSTFLAGS=-A non_camel_case_types");
}