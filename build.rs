use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/fastword.rc");
    println!("cargo:rerun-if-changed=assets/icon/fastword.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
        || env::var_os("CARGO_FEATURE_GUI").is_none()
    {
        return;
    }
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("fastword.res");
    let compiled = ["llvm-rc", "rc.exe"].iter().any(|tool| {
        Command::new(tool)
            .current_dir("assets/icon")
            .arg("/FO").arg(&output).arg("fastword.rc")
            .status().is_ok_and(|status| status.success())
    });
    assert!(compiled, "FastWord icon requires llvm-rc or rc.exe");
    println!("cargo:rustc-link-arg-bin=fastword-gui={}", output.display());
}
