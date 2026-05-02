use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    #[serde(rename = "modelSize")]
    pub model_size: String,
    pub downloaded: u64,
    pub total: u64,
    pub percent: f64,
}

pub async fn download_model(
    app: AppHandle,
    model_size: &str,
    url: &str,
    dest: &PathBuf,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Download client failed: {}", e))?;
    if cancel.load(Ordering::SeqCst) {
        cancel.store(false, Ordering::SeqCst);
        return Err("Download cancelled".into());
    }
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed for {model_size} with status: {} ({url})",
            response.status()
        ));
    }

    let total = response.content_length().unwrap_or(0);

    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let tmp_dest = temp_path_for(dest)?;
    let _ = std::fs::remove_file(&tmp_dest);
    let mut file = std::fs::File::create(&tmp_dest).map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            let _ = std::fs::remove_file(&tmp_dest);
            cancel.store(false, Ordering::SeqCst);
            return Err("Download cancelled".into());
        }

        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                let _ = std::fs::remove_file(&tmp_dest);
                return Err(format!("Download stream error: {}", e));
            }
        };
        if let Err(e) = file.write_all(&chunk) {
            let _ = std::fs::remove_file(&tmp_dest);
            return Err(e.to_string());
        }
        downloaded += chunk.len() as u64;

        let percent = if total > 0 {
            (downloaded as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let payload = DownloadProgress {
            model_size: model_size.to_string(),
            downloaded,
            total,
            percent,
        };
        let _ = app.emit("model:download:progress", payload.clone());
        let _ = app.emit("download-progress", payload);
    }

    if cancel.load(Ordering::SeqCst) {
        let _ = std::fs::remove_file(&tmp_dest);
        cancel.store(false, Ordering::SeqCst);
        return Err("Download cancelled".into());
    }

    if total > 0 && downloaded != total {
        let _ = std::fs::remove_file(&tmp_dest);
        return Err(format!(
            "Download incomplete for {model_size}: {downloaded} of {total} bytes"
        ));
    }

    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    if dest.exists() {
        std::fs::remove_file(dest).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp_dest, dest).map_err(|e| e.to_string())?;
    Ok(())
}

fn temp_path_for(dest: &PathBuf) -> Result<PathBuf, String> {
    let file_name = dest
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "model destination has no file name".to_string())?;
    Ok(dest.with_file_name(format!("{file_name}.part")))
}
