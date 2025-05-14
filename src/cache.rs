use std::{
    env,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::url_entry::UrlEntry;
use once_cell::sync::Lazy;
use tokio::{
    fs::{OpenOptions, read, read_dir, remove_file},
    io::{AsyncWriteExt, BufWriter},
    time::{self, Duration},
};

const CACHE_DIR: &str = "barbara";
const CACHE_TTL_SECS: u64 = 60 * 60 * 24 * 2; // 2 day
static FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);
static CACHE_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut d = env::temp_dir();
    d.push(CACHE_DIR);
    if !d.exists() {
        std::fs::create_dir_all(&d).expect("creating cache dir");
    }
    d
});

/// Creates the cache temp dir if it doesn’t exist yet and returns it.
fn cache_dir() -> &'static PathBuf {
    &*CACHE_PATH
}

async fn cleanup_cache_async() -> std::io::Result<()> {
    let mut rd = read_dir(cache_dir()).await?;
    while let Some(ent) = rd.next_entry().await? {
        let path = ent.path();
        if let Ok(meta) = ent.metadata().await {
            if let Ok(modified) = meta.modified() {
                if modified.elapsed().expect("Unable to cleanup cache due to error in converting system time. Must manually delete cache").as_secs() > CACHE_TTL_SECS {
                    let _ = remove_file(path).await;
                }
            }
        }
    }
    Ok(())
}

// spawn this once at startup:
pub fn spawn_cache_cleaner() {
    tokio::spawn(async {
        let mut interval = time::interval(Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            if let Err(e) = cleanup_cache_async().await {
                eprintln!("cache cleanup error: {}", e);
            }
        }
    });
}

/// Write url entries into a cache file
/// Silently fails if the encoding fails
pub async fn write_to_cache_async(data: Vec<UrlEntry>) -> std::io::Result<()> {
    // 1) build a unique path
    let pid = std::process::id();
    let micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros();
    let count = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = cache_dir().join(format!("barbara_{}_{}_{}.bin", pid, micros, count));

    // 2) async open + buffered write + streaming bincode
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&path)
        .await?;
    let mut writer = BufWriter::new(file);
    let encoding = bincode::encode_to_vec(data, bincode::config::standard())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    writer.write_all(&encoding).await?;
    writer.flush().await?;
    Ok(())
}

/// Reads the most recent cache file and returns its contents as a vector of `UrlEntry`.
/// Passes along the decoding errror if it occurs.
pub async fn read_cache_async() -> std::io::Result<Option<Vec<UrlEntry>>> {
    // find most-recent...
    let mut entries = read_dir(cache_dir()).await?;
    let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;
    while let Some(ent) = entries.next_entry().await? {
        if let Ok(meta) = ent.metadata().await {
            if let Ok(modified) = meta.modified() {
                let path = ent.path();
                if latest.as_ref().map_or(true, |(t, _)| modified > *t) {
                    latest = Some((modified, path));
                }
            }
        }
    }

    if let Some((_, path)) = latest {
        let bytes = read(&path).await?;
        let data: (Vec<UrlEntry>, usize) =
            bincode::decode_from_slice(&bytes, bincode::config::standard())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        if data.1 == 0 {
            Ok(None)
        } else {
            Ok(Some(data.0))
        }
    } else {
        Ok(None)
    }
}
