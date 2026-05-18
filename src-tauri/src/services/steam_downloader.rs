use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::Client;
use sha1::{Digest, Sha1};
use tokio::fs;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::{Mutex, Semaphore};

use crate::services::manifest_code_provider::fetch_manifest_request_code_external;
use crate::services::steam_cdn;
use crate::services::steam_chunks::decode_chunk;
use crate::services::steam_manifest::{decode_manifest, DecodedManifest};
use crate::services::steam_session::{
    discover_cdn_servers, fetch_cdn_auth_token, fetch_manifest_request_code, CdnServer,
    SteamSession,
};

const CHUNK_CONCURRENCY: usize = 32;
const CHUNK_RETRY_COUNT: usize = 8;

#[derive(Clone, Debug)]
pub struct NativeDownloadProgress {
    pub depot_id: u32,
    pub total_chunks: u64,
    pub completed_chunks: u64,
    pub total_bytes: u64,
    pub completed_bytes: u64,
    pub network_bytes: u64,
    pub skipped_chunks: u64,
    pub skipped_bytes: u64,
    pub percent: f64,
}

pub struct NativeDownloadOutcome {
    pub files_written: usize,
    pub bytes_written: u64,
}

pub async fn download_depot_native(
    http: Client,
    session: Arc<SteamSession>,
    app_id: u32,
    depot_id: u32,
    manifest_id: u64,
    depot_key: [u8; 32],
    out_dir: PathBuf,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    progress: impl Fn(NativeDownloadProgress) + Send + Sync + 'static,
) -> Result<NativeDownloadOutcome, String> {
    let servers = discover_cdn_servers(session.clone(), 0, 20).await?;
    if servers.is_empty() {
        return Err("no CDN servers returned by Steam".to_string());
    }
    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }

    let request_code = resolve_manifest_request_code(
        &http,
        session.clone(),
        app_id,
        depot_id,
        manifest_id,
    )
    .await?;

    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }

    let manifest = fetch_manifest_with_fallback(
        &http,
        &servers,
        session.clone(),
        app_id,
        depot_id,
        manifest_id,
        request_code,
        &depot_key,
    )
    .await?;

    download_chunks_from_manifest(
        http,
        session,
        servers,
        app_id,
        depot_id,
        depot_key,
        manifest,
        out_dir,
        cancel,
        pause,
        progress,
    )
    .await
}

pub async fn download_depot_from_local_manifest(
    http: Client,
    session: Arc<SteamSession>,
    app_id: u32,
    depot_id: u32,
    depot_key: [u8; 32],
    manifest_bytes: Vec<u8>,
    out_dir: PathBuf,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    progress: impl Fn(NativeDownloadProgress) + Send + Sync + 'static,
) -> Result<NativeDownloadOutcome, String> {
    let servers = discover_cdn_servers(session.clone(), 0, 20).await?;
    if servers.is_empty() {
        return Err("no CDN servers returned by Steam".to_string());
    }
    let manifest = decode_manifest(&manifest_bytes, &depot_key)?;
    download_chunks_from_manifest(
        http,
        session,
        servers,
        app_id,
        depot_id,
        depot_key,
        manifest,
        out_dir,
        cancel,
        pause,
        progress,
    )
    .await
}

async fn download_chunks_from_manifest(
    http: Client,
    session: Arc<SteamSession>,
    servers: Vec<CdnServer>,
    app_id: u32,
    depot_id: u32,
    depot_key: [u8; 32],
    manifest: DecodedManifest,
    out_dir: PathBuf,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    progress: impl Fn(NativeDownloadProgress) + Send + Sync + 'static,
) -> Result<NativeDownloadOutcome, String> {
    pre_create_files(&manifest, &out_dir).await?;

    let total_chunks: u64 = manifest
        .payload
        .mappings
        .iter()
        .map(|m| m.chunks.len() as u64)
        .sum();
    let total_bytes: u64 = manifest
        .payload
        .mappings
        .iter()
        .flat_map(|m| m.chunks.iter())
        .map(|c| c.cb_original.unwrap_or(0) as u64)
        .sum();

    let progress = Arc::new(progress);
    let completed_chunks = Arc::new(Mutex::new(0u64));
    let completed_bytes = Arc::new(Mutex::new(0u64));
    let network_bytes = Arc::new(Mutex::new(0u64));
    let skipped_chunks = Arc::new(Mutex::new(0u64));
    let skipped_bytes = Arc::new(Mutex::new(0u64));
    let semaphore = Arc::new(Semaphore::new(CHUNK_CONCURRENCY));
    let token_cache: Arc<Mutex<HashMap<String, Option<String>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let mut tasks = FuturesUnordered::new();
    for mapping in &manifest.payload.mappings {
        let Some(relative_path) = mapping.filename.clone() else {
            continue;
        };
        if mapping.flags.unwrap_or(0) & 0x40 != 0 {
            continue;
        }
        let file_path = out_dir.join(sanitise_relative_path(&relative_path));

        for chunk in &mapping.chunks {
            let Some(sha) = chunk.sha.as_ref() else {
                continue;
            };
            let chunk_meta = ChunkJob {
                sha: sha.clone(),
                offset: chunk.offset.unwrap_or(0),
                cb_original: chunk.cb_original.unwrap_or(0),
                cb_compressed: chunk.cb_compressed.unwrap_or(0),
                file_path: file_path.clone(),
            };
            let http = http.clone();
            let session = session.clone();
            let servers = servers.clone();
            let depot_key = depot_key;
            let progress = progress.clone();
            let completed_chunks = completed_chunks.clone();
            let completed_bytes = completed_bytes.clone();
            let network_bytes = network_bytes.clone();
            let skipped_chunks = skipped_chunks.clone();
            let skipped_bytes = skipped_bytes.clone();
            let permit = semaphore.clone();
            let token_cache = token_cache.clone();
            let cancel = cancel.clone();
            let pause = pause.clone();

            tasks.push(tokio::spawn(async move {
                let _permit = permit.acquire_owned().await;
                if cancel.load(Ordering::SeqCst) {
                    return Err("cancelled".to_string());
                }
                while pause.load(Ordering::SeqCst) {
                    if cancel.load(Ordering::SeqCst) {
                        return Err("cancelled".to_string());
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
                let res = process_chunk(
                    http,
                    session,
                    &servers,
                    app_id,
                    depot_id,
                    &depot_key,
                    chunk_meta,
                    token_cache,
                )
                .await;
                if let Ok(ref outcome) = res {
                    let mut cc = completed_chunks.lock().await;
                    *cc += 1;
                    let cc_val = *cc;
                    drop(cc);
                    let mut cb = completed_bytes.lock().await;
                    *cb += outcome.bytes_written;
                    let cb_val = *cb;
                    drop(cb);
                    let mut nb = network_bytes.lock().await;
                    *nb += outcome.network_bytes;
                    let nb_val = *nb;
                    drop(nb);
                    if outcome.from_cache {
                        let mut sc = skipped_chunks.lock().await;
                        *sc += 1;
                        let mut sb = skipped_bytes.lock().await;
                        *sb += outcome.bytes_written;
                    }
                    let skipped_chunks_val = *skipped_chunks.lock().await;
                    let skipped_bytes_val = *skipped_bytes.lock().await;
                    let percent = if total_bytes > 0 {
                        (cb_val as f64) * 100.0 / (total_bytes as f64)
                    } else {
                        0.0
                    };
                    progress(NativeDownloadProgress {
                        depot_id,
                        total_chunks,
                        completed_chunks: cc_val,
                        total_bytes,
                        completed_bytes: cb_val,
                        network_bytes: nb_val,
                        skipped_chunks: skipped_chunks_val,
                        skipped_bytes: skipped_bytes_val,
                        percent,
                    });
                }
                res
            }));
        }
    }

    let mut files_written_unique = std::collections::HashSet::new();
    let mut bytes_written = 0u64;
    while let Some(joined) = tasks.next().await {
        let chunk_outcome = joined
            .map_err(|e| format!("chunk task join failed: {}", e))??;
        bytes_written += chunk_outcome.bytes_written;
        files_written_unique.insert(chunk_outcome.file_path);
    }

    Ok(NativeDownloadOutcome {
        files_written: files_written_unique.len(),
        bytes_written,
    })
}

struct ChunkJob {
    sha: Vec<u8>,
    offset: u64,
    cb_original: u32,
    cb_compressed: u32,
    file_path: PathBuf,
}

struct ChunkOutcome {
    file_path: PathBuf,
    bytes_written: u64,
    network_bytes: u64,
    from_cache: bool,
}

async fn process_chunk(
    http: Client,
    session: Arc<SteamSession>,
    servers: &[CdnServer],
    app_id: u32,
    depot_id: u32,
    depot_key: &[u8; 32],
    job: ChunkJob,
    token_cache: Arc<Mutex<HashMap<String, Option<String>>>>,
) -> Result<ChunkOutcome, String> {
    let sha_hex = hex::encode(&job.sha);

    if job.cb_original > 0 {
        if let Ok(existing) = read_existing_chunk(&job.file_path, job.offset, job.cb_original).await
        {
            let mut hasher = Sha1::new();
            hasher.update(&existing);
            if hasher.finalize().to_vec() == job.sha {
                return Ok(ChunkOutcome {
                    file_path: job.file_path,
                    bytes_written: job.cb_original as u64,
                    network_bytes: 0,
                    from_cache: true,
                });
            }
        }
    }

    let mut last_err: Option<String> = None;
    for attempt in 0..CHUNK_RETRY_COUNT {
        if attempt > 0 {
            let delay_ms = chunk_retry_backoff_ms(attempt);
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
        let server = pick_server(servers, attempt);
        let token = get_or_fetch_token(
            &token_cache,
            session.clone(),
            &server.host,
            app_id,
            depot_id,
        )
        .await;
        let encrypted = match steam_cdn::fetch_encrypted_chunk(
            &http,
            &server.host,
            depot_id,
            &sha_hex,
            token.as_deref(),
            !server.https_support.is_empty() && server.https_support != "none",
        )
        .await
        {
            Ok(b) if b.len() as u32 == job.cb_compressed || job.cb_compressed == 0 => b,
            Ok(b) => {
                last_err = Some(format!(
                    "chunk size mismatch: got {} bytes, expected {}",
                    b.len(),
                    job.cb_compressed
                ));
                continue;
            }
            Err(e) => {
                if is_hard_chunk_error(&e) {
                    return Err(e);
                }
                last_err = Some(e);
                continue;
            }
        };
        let network_bytes = encrypted.len() as u64;

        let decoded = match decode_chunk(&encrypted, depot_key, job.cb_original) {
            Ok(d) => d,
            Err(e) => {
                last_err = Some(e);
                continue;
            }
        };

        let actual_sha = {
            let mut hasher = Sha1::new();
            hasher.update(&decoded);
            hasher.finalize().to_vec()
        };
        if actual_sha != job.sha {
            last_err = Some(format!(
                "chunk SHA mismatch: got {} expected {}",
                hex::encode(&actual_sha),
                sha_hex
            ));
            continue;
        }

        if let Some(parent) = job.file_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("create dir failed: {}", e))?;
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&job.file_path)
            .await
            .map_err(|e| format!("open file failed: {}", e))?;
        let _ = file
            .seek(std::io::SeekFrom::Start(job.offset))
            .await
            .map_err(|e| format!("seek failed: {}", e))?;
        file.write_all(&decoded)
            .await
            .map_err(|e| format!("write failed: {}", e))?;

        return Ok(ChunkOutcome {
            file_path: job.file_path,
            bytes_written: decoded.len() as u64,
            network_bytes,
            from_cache: false,
        });
    }
    Err(last_err.unwrap_or_else(|| "chunk download exhausted retries".to_string()))
}

async fn read_existing_chunk(path: &Path, offset: u64, len: u32) -> Result<Vec<u8>, String> {
    use tokio::io::AsyncReadExt;
    if !path.exists() {
        return Err("file missing".to_string());
    }
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|e| format!("metadata: {}", e))?;
    if metadata.len() < offset + len as u64 {
        return Err("file too short".to_string());
    }
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| format!("open: {}", e))?;
    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .map_err(|e| format!("seek: {}", e))?;
    let mut buf = vec![0u8; len as usize];
    file.read_exact(&mut buf)
        .await
        .map_err(|e| format!("read: {}", e))?;
    Ok(buf)
}

async fn get_or_fetch_token(
    cache: &Mutex<HashMap<String, Option<String>>>,
    session: Arc<SteamSession>,
    host: &str,
    app_id: u32,
    depot_id: u32,
) -> Option<String> {
    let key = format!("{}:{}:{}", host, app_id, depot_id);
    {
        let guard = cache.lock().await;
        if let Some(entry) = guard.get(&key) {
            return entry.clone();
        }
    }
    let token = fetch_cdn_auth_token(session, host, app_id, depot_id)
        .await
        .unwrap_or(None);
    let mut guard = cache.lock().await;
    guard.entry(key).or_insert_with(|| token.clone());
    token
}

async fn resolve_manifest_request_code(
    http: &Client,
    session: Arc<SteamSession>,
    app_id: u32,
    depot_id: u32,
    manifest_id: u64,
) -> Result<u64, String> {
    match fetch_manifest_request_code(session, app_id, depot_id, manifest_id).await {
        Ok(code) if code != 0 => return Ok(code),
        Ok(_) => {
            eprintln!(
                "[manifest-code] Steam returned request_code=0 for depot {}, falling back to external providers",
                depot_id
            );
        }
        Err(e) => {
            eprintln!(
                "[manifest-code] Steam GetManifestRequestCode failed for depot {} ({}), falling back to external providers",
                depot_id, e
            );
        }
    }
    match fetch_manifest_request_code_external(http, manifest_id).await {
        Ok(resolved) => {
            eprintln!(
                "[manifest-code] resolved depot {} via {}",
                depot_id,
                resolved.source.label()
            );
            Ok(resolved.request_code)
        }
        Err(e) => Err(e),
    }
}

async fn fetch_manifest_with_fallback(
    http: &Client,
    servers: &[CdnServer],
    session: Arc<SteamSession>,
    app_id: u32,
    depot_id: u32,
    manifest_id: u64,
    request_code: u64,
    depot_key: &[u8; 32],
) -> Result<DecodedManifest, String> {
    let mut last_err: Option<String> = None;
    for (idx, server) in servers.iter().enumerate().take(5) {
        let token = fetch_cdn_auth_token(session.clone(), &server.host, app_id, depot_id)
            .await
            .unwrap_or(None);
        let use_https = !server.https_support.is_empty() && server.https_support != "none";
        match steam_cdn::fetch_decoded_manifest(
            http,
            &server.host,
            depot_id,
            manifest_id,
            request_code,
            token.as_deref(),
            depot_key,
            use_https,
        )
        .await
        {
            Ok(m) => return Ok(m),
            Err(e) => {
                last_err = Some(format!("server #{} ({}) failed: {}", idx, server.host, e));
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "no servers tried".to_string()))
}

fn pick_server<'a>(servers: &'a [CdnServer], attempt: usize) -> &'a CdnServer {
    &servers[attempt % servers.len()]
}

fn chunk_retry_backoff_ms(attempt: usize) -> u64 {
    let base = 250u64 << attempt.min(7);
    base.min(15_000)
}

fn is_hard_chunk_error(err: &str) -> bool {
    err.contains(" 400 ")
        || err.contains(" 401 ")
        || err.contains(" 403 ")
        || err.contains(" 404 ")
        || err.contains(" 410 ")
}

fn sanitise_relative_path(p: &str) -> String {
    p.replace('\\', "/")
}

async fn pre_create_files(manifest: &DecodedManifest, out_dir: &Path) -> Result<(), String> {
    for mapping in &manifest.payload.mappings {
        let Some(rel) = mapping.filename.as_ref() else {
            continue;
        };
        let target = out_dir.join(sanitise_relative_path(rel));
        if mapping.flags.unwrap_or(0) & 0x40 != 0 {
            fs::create_dir_all(&target)
                .await
                .map_err(|e| format!("create dir failed: {}", e))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("create parent dir failed: {}", e))?;
        }
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&target)
            .await
            .map_err(|e| format!("pre-create file failed: {}", e))?;
        let expected = mapping.size.unwrap_or(0);
        if expected > 0 {
            file.set_len(expected)
                .await
                .map_err(|e| format!("set_len failed: {}", e))?;
        }
    }
    Ok(())
}

