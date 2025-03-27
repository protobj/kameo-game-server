use regex::Regex;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
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
    if let Ok(ok) = fs::exists("./src/client/cmd.proto".to_string()) {
        if ok {
            fs::remove_file("./src/client/cmd.proto".to_string()).unwrap();
        }
    }
    if let Ok(ok) = fs::exists("./src/client/error.proto".to_string()) {
        if ok {
            fs::remove_file("./src/client/error.proto".to_string()).unwrap();
        }
    }
    let base_dir = "./src";
    let proto_dir = Path::new(base_dir);
    let mut proto_files = find_proto_files(proto_dir)?;

    check_code(proto_files.clone(), "cmd.txt", true)?;
    check_code(proto_files.clone(), "error.txt", false)?;

    proto_files.push("./src/client/cmd.proto".to_string());
    proto_files.push("./src/client/error.proto".to_string());

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
    // 写入 lib.rs
    fs::write(mod_path, mod_content)?;
    Ok(())
}
fn check_code(
    proto_files: Vec<String>,
    code_file: &str,
    cmd: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    //先读出模块命令码范围
    let file = File::open(code_file)?;
    let reader = BufReader::new(file);
    let mut range_map: HashMap<String, (u16, u16)> = HashMap::new();
    let mut unique_codes = HashSet::new();
    let mut code_map: BTreeMap<u16, String> = BTreeMap::new();
    let re = Regex::new(r"(\w+)\s*=\s*\[\s*(\d+)\s*,\s*(\d+)\s*]").unwrap();
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        if let Some(captures) = re.captures(&line) {
            range_map.insert(
                captures[1].to_string(),
                (captures[2].parse()?, captures[3].parse()?),
            );
        }
    }
    let re_code = Regex::new(r"(\w+)\s*=\s*\s*(\d+)\s*").unwrap();
    for file in proto_files {
        if !file.ends_with("_cmd.proto") {
            continue;
        }
        let file_name = Path::new(&file).file_name().unwrap().to_str().unwrap();
        let enum_name = snake_to_camel(&file_name.replace(".proto", ""));
        let enum_name = if cmd {
            enum_name
        } else {
            enum_name.replace("Cmd", "Error")
        };
        let range = range_map.get(&enum_name);
        let range = match range {
            Some((start, end)) => (start, end),
            None => panic!(
                "{} is not defined in range map file:{}",
                enum_name, code_file
            ),
        };

        // 1. 读取文件内容
        let content = fs::read_to_string(file)?;

        // 2. 定义正则表达式（启用多行模式）
        let re_enum_struct =
            Regex::new(&r#"enum\s+LoginCmd\s*\{([^}]+)\}"#.replace("LoginCmd", &enum_name))
                .unwrap();

        // 3. 提取匹配的枚举块
        if let Some(caps) = re_enum_struct.captures(&content) {
            let enum_content = caps.get(1).unwrap().as_str();
            for code in re_code.captures_iter(enum_content) {
                let enum_unit_name = &code[1];
                let code = &code[2];
                let code: u16 = code.parse::<u16>()?;
                if code == 0 {
                    continue;
                }
                if code < *range.0 || code > *range.1 {
                    panic!(
                        "{} is not defined in range map code:{} range:{:?}",
                        enum_name, code, range
                    );
                }
                if !unique_codes.insert(code) {
                    panic!("{} is already defined code:{}", enum_name, code);
                }
                code_map.insert(code, enum_unit_name.to_string());
            }
        }
    }
    //生成合并文件
    let mut proto_content = String::new();
    proto_content.push_str("syntax = \"proto3\";\n\n");

    if cmd {
        proto_content.push_str("package cmd;\n\n");
        proto_content.push_str("enum Cmd{\n");
        proto_content.push_str("    CmdNone = 0;\n");
        for x in code_map {
            proto_content.push_str(&format!("    Cmd{}={};\n", x.1, x.0));
        }
    } else {
        proto_content.push_str("package error;\n\n");
        proto_content.push_str("enum Error{\n");
        proto_content.push_str("    Ok = 0;\n");
        for x in code_map {
            proto_content.push_str(&format!("    Error{}={};\n", x.1, x.0));
        }
    }
    proto_content.push_str("}\n");
    if cmd {
        fs::write(Path::new("./src/client/cmd.proto"), proto_content)?;
    } else {
        fs::write(Path::new("./src/client/error.proto"), proto_content)?;
    }
    Ok(())
}
fn _camel_to_snake(camel: &str) -> String {
    let mut snake = String::new();
    for (i, c) in camel.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            snake.push('_');
        }
        snake.push(c.to_ascii_lowercase());
    }
    snake
}
fn snake_to_camel(snake: &str) -> String {
    let mut camel = String::new();
    let mut uppercase_ix = 0usize;
    for (i, c) in snake.chars().enumerate() {
        if i == uppercase_ix {
            camel.push(c.to_uppercase().next().unwrap());
            continue;
        }
        if c == '_' {
            uppercase_ix = i + 1;
        } else {
            camel.push(c);
        }
    }
    camel
}
