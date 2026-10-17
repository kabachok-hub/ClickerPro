use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=app_icon.ico");
    println!("cargo:rerun-if-changed=app.rc");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let out_dir = env::var("OUT_DIR").unwrap();
        let res_path = format!("{}/app_icon.res", out_dir);
        let rc_status = Command::new("windres")
            .args(&["-i", "app.rc", "-O", "coff", "-o", &res_path])
            .status();

        if let Ok(status) = rc_status {
            if status.success() {
                println!("cargo:rustc-link-arg={}", res_path);
            }
        }
    }
}
