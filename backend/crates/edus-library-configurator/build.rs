use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("build output"));
    let rc = out.join("configurator.rc");
    let res = out.join("configurator.res");
    fs::write(
        &rc,
        format!(
            "1 24 \"{}\"\n",
            root.join("app.manifest")
                .display()
                .to_string()
                .replace('\\', "\\\\")
        ),
    )
    .expect("write resource source");
    let status = Command::new("rc.exe")
        .arg("/nologo")
        .arg("/fo")
        .arg(&res)
        .arg(&rc)
        .status()
        .expect("Windows SDK rc.exe required in Developer environment");
    assert!(
        status.success(),
        "compile native Configurator administrator manifest"
    );
    println!(
        "cargo:rustc-link-arg-bin=EDUSTerminalConfigurator={}",
        res.display()
    );
}
