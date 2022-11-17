mod error;

use std::io;
use std::env;
use std::io::Read;
use std::path;
use std::process::ExitStatus;
use clap::Parser;
use toml_edit::easy;
use serde::Deserialize;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, value_name = "FILE")]
    bin: Option<String>
}

#[derive(Deserialize)]
struct TomlConfig {
    package: TomlPackage,
    bin: Option<Vec<TomlBin>>
}

#[derive(Deserialize)]
struct TomlPackage {
    name: Option<String>
}

#[derive(Deserialize)]
struct TomlBin {
    name: Option<String>
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let project_root = get_project_root()?;
    let project_root = project_root.as_path();
    let qemu_path = get_qemu_executable()?;
    let ovmf_path = get_ovmf(project_root)?;

    // 実行するアプリケーションを選択する
    let cargo_toml_path = project_root.join("Cargo.toml");
    let mut cargo_toml = std::fs::File::open(cargo_toml_path.as_path())?;
    let mut toml = String::new();
    cargo_toml.read_to_string(&mut toml);
    let app_name = find_binary_name(&args.bin, toml.as_str())?;
    let app_path = get_uefi_app(project_root, app_name.as_str())?;

    // UEFIアプリケーションを配置するための一時ディレクトリを作成
    let uefi_root = env::temp_dir().join("UEFI");
    let uefi_app_dir = env::temp_dir().join("UEFI").join("EFI").join("BOOT");
    std::fs::create_dir_all(uefi_app_dir.as_path())?;

    // 作成したディレクトリにUEFIアプリケーションを配置
    let uefi_app_path = uefi_app_dir.join("BOOTX64.EFI");  
    std::fs::copy(app_path, uefi_app_path)?;

    // QEMUを実行
    run_qemu(qemu_path.as_path(), ovmf_path.as_path(), uefi_root.as_path())?;

    Ok(())
}

fn get_project_root() -> Result<path::PathBuf, io::Error> {
    let cargo_name = "Cargo.toml";
    let current_dir = env::current_dir()?;
    let mut ancestors = current_dir.ancestors();

    ancestors.find(|path| path.join(cargo_name).is_file())
        .map(|p| p.to_path_buf())
        .ok_or(io::Error::new(io::ErrorKind::NotFound, "project root directory not found"))
}

fn get_qemu_executable() -> Result<path::PathBuf, io::Error> {
    let qemu_name = "qemu-system-x86_64";

    let exec_path = env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).filter_map(|path| {
            let full_path = path.join(qemu_name);
            if full_path.is_file() {
                Some(full_path)
            } else {
                None
            }
        }).next()
    });

    exec_path.ok_or(io::Error::new(io::ErrorKind::NotFound, format!("{} is not found", qemu_name)))
}

fn get_ovmf(project_root_dir: &path::Path) -> Result<path::PathBuf, io::Error> {
    let ovmf_name = "OVMF.fd";

    let ovmf_path = project_root_dir.join(ovmf_name); 
    if ovmf_path.is_file() {
        Ok(ovmf_path)
    } else {
        Err(io::Error::new(io::ErrorKind::NotFound, format!("{} is not found", ovmf_name)))
    }
}

fn get_uefi_app(project_root_dir: &path::Path, app_name: &str) -> Result<path::PathBuf, io::Error> {
    let mut app_path = project_root_dir.to_path_buf();
    app_path.push("target");
    app_path.push("x86_64-unknown-uefi");
    app_path.push("debug");
    app_path.push(format!("{}.efi", app_name));

    if app_path.is_file() {
        Ok(app_path)
    } else {
        Err(io::Error::new(io::ErrorKind::NotFound, format!("{} is not found", app_name)))
    }
}

fn run_qemu(qemu: &path::Path, ovmf: &path::Path, uefi_root: &path::Path) -> Result<ExitStatus, io::Error> { 
    let mut process = std::process::Command::new(qemu.display().to_string())
        .arg("-drive")
        .arg(format!("if=pflash,format=raw,readonly=on,file={}", ovmf.display())) 
        .arg("-drive")
        .arg(format!("format=raw,file=fat:rw:{}", uefi_root.display()))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

   process.wait()
}

fn find_binary_name(app_name: &Option<String>, toml: &str) -> Result<String, Box<dyn std::error::Error>> {
    let names = get_binary_name(toml)?;
    
    let result = match &app_name {
        None if names.len() == 1 => Ok(names[0].clone()),
        None => Err(crate::error::Error::new(
            error::ErrorKind::NotAbleDetermineBinary, 
            format!("multiple candidates exists, not ablt to determine. {:?}", names)
        )),
        Some(name) if names.contains(&name) => Ok(name.clone()),
        Some(name) => Err(error::Error::new(
            error::ErrorKind::BinaryNotFound,
            format!("binary {} is not found", name)
        ))
    };

    result.map_err(|e| Box::<dyn std::error::Error>::from(e))
}

fn get_binary_name(toml: &str) -> Result<Vec<String>, toml_edit::de::Error> {
    let toml = easy::from_str::<TomlConfig>(toml)?;
    let bin_names: Vec<String> = toml.bin
        .unwrap_or(Vec::new())
        .into_iter()
        .filter_map(|b| b.name)
        .collect();
    let names = if bin_names.is_empty() {
        toml.package.name.into_iter().collect()
    } else {
        bin_names
    };
    
    Ok(names)
}

#[cfg(test)]
mod test {
    use toml_edit::easy;

    use crate::get_binary_name;

    #[test]
    fn parse_one_bin_pattern() {
        let toml = r#"
        [package] 
        name = "hoge"
        
        [[bin]]
        name = "fuga"
        path = "src/fuga/main.rs"
        "#;
        
        let names = get_binary_name(toml).unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "fuga");
    }

    #[test]
    fn parse_multiple_bin_pattern() {
        let toml = r#"
        [package]
        name = "hoge" 

        [[bin]]
        name = "hogehoge"
        path = "src/hogehoge/main.rs"

        [[bin]]
        name = "fugafuga"
        path = "src/fugafuga/main.rs" 
        "#;

        let names = get_binary_name(toml).unwrap();
        assert_eq!(names.len(), 2);
        assert_eq!(names[0], "hogehoge");
        assert_eq!(names[1], "fugafuga"); 
    }

    #[test]
    fn parse_package_pattern() {
        let toml = r#"
        [package]
        name = "hoge"
        
        [dependencies]
        fuga = "0.1.0"
        "#;

        let names = get_binary_name(toml).unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "hoge");
    }
}