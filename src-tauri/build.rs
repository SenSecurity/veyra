use std::fs;
use std::path::PathBuf;

fn main() {
    tauri_build::build();

    #[cfg(target_os = "windows")]
    copy_whisper_dlls();
}

#[cfg(target_os = "windows")]
fn copy_whisper_dlls() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let binaries_dir = manifest_dir.join("binaries");

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let target_dir = manifest_dir.join("target").join(&profile);

    let dlls = [
        "whisper.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
        "ggml-blas.dll",
        "libopenblas.dll",
    ];

    let _ = fs::create_dir_all(&target_dir);
    for dll in dlls {
        let src = binaries_dir.join(dll);
        let dst = target_dir.join(dll);
        if src.exists() {
            let _ = fs::copy(&src, &dst);
            println!("cargo:rerun-if-changed={}", src.display());
        }
    }
}
