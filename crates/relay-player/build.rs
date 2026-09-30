//! On Windows: the icon, the manifest (DPI awareness) and the version info.

use std::path::{Path, PathBuf};

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let icon = dir.join("../../src-tauri/icons/icon.ico");
    let manifest = dir.join("player.manifest");
    for p in [&icon, &manifest] {
        println!("cargo:rerun-if-changed={}", p.display());
    }
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let numbers: Vec<&str> = version.split(['.', '-']).take(3).collect();
    let comma = format!("{},0", numbers.join(","));
    let path = |p: &Path| p.display().to_string().replace('\\', "/");
    let rc = format!(
        r#"1 ICON "{icon}"
1 24 "{manifest}"
1 VERSIONINFO
FILEVERSION {comma}
PRODUCTVERSION {comma}
FILEOS 0x40004
FILETYPE 1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "HappyRave"
      VALUE "FileDescription", "Relay macro player"
      VALUE "ProductName", "Relay"
      VALUE "FileVersion", "{version}"
      VALUE "ProductVersion", "{version}"
      VALUE "LegalCopyright", "MIT License"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        icon = path(&icon),
        manifest = path(&manifest),
    );
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("player.rc");
    std::fs::write(&out, rc).unwrap();
    embed_resource::compile(&out, embed_resource::NONE).manifest_required().unwrap();
}
