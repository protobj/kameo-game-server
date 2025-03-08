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
            proto_files.push(path.to_str().unwrap().to_owned());
        }
    }
    Ok(proto_files)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = Path::new("./pb");
    let proto_files = find_proto_files(proto_dir)?;

    let out_dir = "gen/proto/src";
    fs::remove_dir_all(out_dir)?;
    fs::create_dir_all(out_dir)?;
    prost_build::Config::new()
        .out_dir(out_dir)
        .format(true)
        .compile_protos(&proto_files, &["./pb"])?;
    // 2. 生成 mod.rs
    let mod_path = PathBuf::from(out_dir).join("lib.rs");
    let mut mod_content = String::new();

    // 扫描生成的 .rs 文件（排除 mod.rs）
    for entry in fs::read_dir(out_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("rs")
            && path.file_name().and_then(|s| s.to_str()) != Some("lib.rs")
        {
            if let Some(module_name) = path.file_stem().and_then(|s| s.to_str()) {
                mod_content.push_str(&format!("pub mod {};\n", module_name));
            }
        }
    }
    // 写入 mod.rs
    fs::write(mod_path, mod_content).unwrap();
    Ok(())
}
