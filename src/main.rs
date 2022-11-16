use std::io;
use std::env;
use std::path;
use std::process::{ExitStatus, ExitCode};

fn main() -> Result<(), io::Error> {
    let project_root = get_project_root()?.as_path();
    let qemu_path = get_qemu_executable()?;
    let ovmf_path = get_ovmf(project_root)?;
    let app_path = get_uefi_app(project_root, &app_name)?;

    // UEFIアプリケーションを配置するための一時ディレクトリを作成
    let mut uefi_app_dir = env::temp_dir();
    uefi_app_dir.push("UEFI");
    let uefi_root = uefi_app_dir.clone().as_path();
    uefi_app_dir.push("EFI");
    uefi_app_dir.push("BOOT"); 
    std::fs::create_dir_all(uefi_app_dir);

    // 作成したディレクトリにUEFIアプリケーションを配置
    uefi_app_dir.push("BOOTX64.EFI");
    std::fs::copy(app_path, uefi_app_dir)?;

    // QEMUを実行
    run_qemu(qemu_path.as_path(), ovmf_path.as_path(), uefi_root)?;

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
    app_path.push("debug");
    app_path.push(app_name);

    if app_path.is_file() {
        Ok(app_path)
    } else {
        Err(io::Error::new(io::ErrorKind::NotFound, format!("{} is not found", app_name)))
    }
}

fn run_qemu(qemu: &path::Path, ovmf: &path::Path, uefi_root: &path::Path) -> Result<ExitStatus, io::Error> { 
    let process = std::process::Command::new(qemu.display().to_string())
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