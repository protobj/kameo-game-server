use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
fn find_proto_files(dir: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut proto_files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // 递归处理子目录
            proto_files.extend(find_proto_files(&path)?);
        } else if path.extension().and_then(|s| s.to_str()) == Some("proto") {
            let proto_file = path.to_str().unwrap().to_owned();
            println!("cargo:rerun-if-changed={proto_file}");
            proto_files.push(proto_file);
        }
    }
    Ok(proto_files)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = "./src";
    let proto_dir = Path::new(base_dir);
    let proto_files = find_proto_files(proto_dir)?;

    prost_build::Config::new()
        .bytes(&["."])
        .compile_protos(&proto_files, &[base_dir])?;
    // 2. 生成 mod.rs
    let mod_path = PathBuf::from(base_dir).join("lib.rs");
    let mut mod_content = String::new();

    let modules: Vec<_> = proto_files
        .iter()
        .map(|file| {
            let file_name = Path::new(file).file_stem().unwrap().to_str().unwrap();
            let module_name = file_name.replace("-", "_").replace("/", "_");
            module_name
        })
        .collect();
    for module in modules {
        writeln!(&mut mod_content, "pub mod {} {{", module)?;
        writeln!(
            &mut mod_content,
            "    include!(concat!(env!(\"OUT_DIR\"), \"/{}.rs\"));",
            module
        )?;
        writeln!(&mut mod_content, "}}")?;
    }
    // 写入 mod.rs
    fs::write(mod_path, mod_content).unwrap();
    Ok(())
}
