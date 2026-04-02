use crate::core::{info, warn};
use crate::types::AppError;
use serde::Deserialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::{
    SHEmptyRecycleBinW, SHQueryRecycleBinW, SHQUERYRBINFO, SHERB_NOCONFIRMATION,
    SHERB_NOPROGRESSUI, SHERB_NOSOUND,
};

const WINDOWS_TEMP_DIR: &str = r"C:\Windows\Temp";
const WINDOWS_PREFETCH_DIR: &str = r"C:\Windows\Prefetch";
const WINDOWS_UPDATE_DOWNLOAD_DIR: &str = r"C:\Windows\SoftwareDistribution\Download";
const WINDOWS_WER_REPORT_ARCHIVE_DIR: &str = r"C:\ProgramData\Microsoft\Windows\WER\ReportArchive";
const WINDOWS_WER_REPORT_QUEUE_DIR: &str = r"C:\ProgramData\Microsoft\Windows\WER\ReportQueue";
const WINDOWS_CBS_LOGS_DIR: &str = r"C:\Windows\Logs\CBS";
const NVIDIA_INSTALLER_DIR: &str = r"C:\NVIDIA\DisplayDriver";
const AMD_INSTALLER_DIR: &str = r"C:\AMD";
const WINDOWS_MEMORY_DUMP_FILE: &str = r"C:\Windows\Memory.dmp";
const PROGRAMDATA_BATTLENET_CACHE_DIR: &str = r"C:\ProgramData\Battle.net\Cache";

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeepPurgeConfig {
    pub(crate) windows: bool,
    pub(crate) gpu: bool,
    pub(crate) browsers: bool,
    pub(crate) apps: bool,
    pub(crate) dev: bool,
}

impl Default for DeepPurgeConfig {
    fn default() -> Self {
        Self {
            windows: true,
            gpu: true,
            browsers: true,
            apps: true,
            dev: true,
        }
    }
}

impl DeepPurgeConfig {
    fn any_enabled(self) -> bool {
        self.windows || self.gpu || self.browsers || self.apps || self.dev
    }
}

fn is_skippable_fs_error(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied | io::ErrorKind::WouldBlock
    ) || matches!(
        error.raw_os_error(),
        Some(2) | Some(3) | Some(5) | Some(32) | Some(33) | Some(145)
    )
}

fn log_skip(action: &str, path: &Path, error: &io::Error) {
    if error.kind() == io::ErrorKind::NotFound || matches!(error.raw_os_error(), Some(2) | Some(3))
    {
        return;
    }
    warn!(
        "deep purge skipped {} on '{}': {}",
        action,
        path.display(),
        error
    );
}

fn file_len(path: &Path) -> u64 {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata.len(),
        _ => 0,
    }
}

fn remove_file_best_effort(path: &Path) -> u64 {
    let bytes = file_len(path);
    match fs::remove_file(path) {
        Ok(()) => bytes,
        Err(error) if is_skippable_fs_error(&error) => {
            log_skip("file", path, &error);
            0
        }
        Err(error) => {
            warn!(
                "deep purge file removal failed for '{}': {}",
                path.display(),
                error
            );
            0
        }
    }
}

fn purge_file_target_best_effort(path: &Path) -> u64 {
    let bytes = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata.len(),
        Ok(_) => return 0,
        Err(error) if is_skippable_fs_error(&error) => {
            return 0;
        }
        Err(error) => {
            warn!(
                "deep purge file target metadata failed for '{}': {}",
                path.display(),
                error
            );
            return 0;
        }
    };

    match fs::remove_file(path) {
        Ok(()) => bytes,
        Err(error) if is_skippable_fs_error(&error) => {
            0
        }
        Err(error) => {
            warn!(
                "deep purge file target removal failed for '{}': {}",
                path.display(),
                error
            );
            0
        }
    }
}

fn remove_symlink_best_effort(path: &Path) -> u64 {
    match fs::remove_file(path) {
        Ok(()) => 0,
        Err(remove_file_error) => match fs::remove_dir(path) {
            Ok(()) => 0,
            Err(remove_dir_error) => {
                if is_skippable_fs_error(&remove_file_error) || is_skippable_fs_error(&remove_dir_error)
                {
                    log_skip("symlink", path, &remove_dir_error);
                    0
                } else {
                    warn!(
                        "deep purge symlink removal failed for '{}': file_error='{}', dir_error='{}'",
                        path.display(),
                        remove_file_error,
                        remove_dir_error
                    );
                    0
                }
            }
        },
    }
}

fn purge_entry_recursive(path: &Path) -> u64 {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if is_skippable_fs_error(&error) => {
            log_skip("metadata", path, &error);
            return 0;
        }
        Err(error) => {
            warn!(
                "deep purge metadata read failed for '{}': {}",
                path.display(),
                error
            );
            return 0;
        }
    };

    if metadata.file_type().is_symlink() {
        return remove_symlink_best_effort(path);
    }

    if metadata.is_dir() {
        let freed = purge_children_only(path);
        match fs::remove_dir(path) {
            Ok(()) => {}
            Err(error) if is_skippable_fs_error(&error) => {
                log_skip("directory", path, &error);
            }
            Err(error) => {
                warn!(
                    "deep purge directory removal failed for '{}': {}",
                    path.display(),
                    error
                );
            }
        }
        return freed;
    }

    remove_file_best_effort(path)
}

fn purge_children_only(root: &Path) -> u64 {
    if !root.exists() {
        return 0;
    }

    let mut freed = 0u64;
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if is_skippable_fs_error(&error) => {
            log_skip("read_dir", root, &error);
            return 0;
        }
        Err(error) => {
            warn!(
                "deep purge read_dir failed for '{}': {}",
                root.display(),
                error
            );
            return 0;
        }
    };

    for entry_result in entries {
        match entry_result {
            Ok(entry) => {
                freed = freed.saturating_add(purge_entry_recursive(&entry.path()));
            }
            Err(error) => {
                warn!(
                    "deep purge entry iteration failed for '{}': {}",
                    root.display(),
                    error
                );
            }
        }
    }

    freed
}

fn estimate_tree_file_bytes(root: &Path) -> u64 {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(_) => return 0,
    };

    if metadata.file_type().is_symlink() {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    if !metadata.is_dir() {
        return 0;
    }

    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            total = total.saturating_add(estimate_tree_file_bytes(&entry.path()));
        }
    }
    total
}

fn purge_root_and_recreate(root: &Path) -> u64 {
    if !root.exists() {
        return 0;
    }

    let estimated = estimate_tree_file_bytes(root);
    match fs::remove_dir_all(root) {
        Ok(()) => {
            if let Err(error) = fs::create_dir_all(root) {
                warn!(
                    "deep purge failed to recreate '{}': {}",
                    root.display(),
                    error
                );
            }
            estimated
        }
        Err(error) => {
            warn!(
                "deep purge root delete failed for '{}': {}. Falling back to children-only purge.",
                root.display(),
                error
            );
            let freed = purge_children_only(root);
            if !root.exists() {
                if let Err(recreate_error) = fs::create_dir_all(root) {
                    warn!(
                        "deep purge fallback failed to recreate '{}': {}",
                        root.display(),
                        recreate_error
                    );
                }
            }
            freed
        }
    }
}

fn is_benign_recycle_bin_hresult(code: i32) -> bool {
    matches!(
        code as u32,
        0x8000_4005 | // E_FAIL (often returned for empty/unsupported recycle-bin edge-cases)
        0x8007_0002 | // HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND)
        0x8007_0003 // HRESULT_FROM_WIN32(ERROR_PATH_NOT_FOUND)
    )
}

fn query_recycle_bin_bytes() -> u64 {
    let mut info = SHQUERYRBINFO::default();
    info.cbSize = std::mem::size_of::<SHQUERYRBINFO>() as u32;
    unsafe {
        match SHQueryRecycleBinW(PCWSTR::null(), &mut info) {
            Ok(()) => info.i64Size.max(0) as u64,
            Err(error) if is_benign_recycle_bin_hresult(error.code().0) => 0,
            Err(error) => {
                warn!("deep purge recycle bin size query failed: {}", error);
                0
            }
        }
    }
}

fn empty_recycle_bin_best_effort() -> u64 {
    let before_bytes = query_recycle_bin_bytes();
    let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
    unsafe {
        if let Err(error) = SHEmptyRecycleBinW(None, PCWSTR::null(), flags) {
            if is_benign_recycle_bin_hresult(error.code().0) {
                return 0;
            }
            warn!("deep purge recycle bin empty failed: {}", error);
            return 0;
        }
    }
    let after_bytes = query_recycle_bin_bytes();
    before_bytes.saturating_sub(after_bytes)
}

pub(crate) fn run_deep_purge(config: DeepPurgeConfig) -> Result<u64, AppError> {
    if !config.any_enabled() {
        return Ok(0);
    }

    let mut total_freed = 0u64;
    let local_app_data = std::env::var("LOCALAPPDATA").ok().map(PathBuf::from);
    let app_data = std::env::var("APPDATA").ok().map(PathBuf::from);
    let user_profile = std::env::var("USERPROFILE").ok().map(PathBuf::from);

    if config.windows {
        let mut targets = vec![
            std::env::temp_dir(),
            PathBuf::from(WINDOWS_TEMP_DIR),
            PathBuf::from(WINDOWS_UPDATE_DOWNLOAD_DIR),
            PathBuf::from(WINDOWS_WER_REPORT_ARCHIVE_DIR),
            PathBuf::from(WINDOWS_WER_REPORT_QUEUE_DIR),
            PathBuf::from(WINDOWS_CBS_LOGS_DIR),
        ];

        if let Some(local) = &local_app_data {
            targets.push(local.join("CrashDumps"));
        } else {
            warn!("deep purge skipped windows LOCALAPPDATA targets: LOCALAPPDATA is unavailable");
        }

        if let Some(app) = &app_data {
            targets.push(app.join("Microsoft").join("Windows").join("Recent"));
        } else {
            warn!("deep purge skipped windows APPDATA targets: APPDATA is unavailable");
        }

        if let Some(user) = &user_profile {
            let cryptnet_url_cache = user
                .join("AppData")
                .join("LocalLow")
                .join("Microsoft")
                .join("CryptnetUrlCache");
            targets.push(cryptnet_url_cache.join("Content"));
            targets.push(cryptnet_url_cache.join("MetaData"));
        } else {
            warn!("deep purge skipped windows USERPROFILE targets: USERPROFILE is unavailable");
        }

        for path in &targets {
            info!("deep purge start: {}", path.display());
            total_freed = total_freed.saturating_add(purge_children_only(path));
        }

        total_freed = total_freed.saturating_add(purge_file_target_best_effort(Path::new(
            WINDOWS_MEMORY_DUMP_FILE,
        )));
        total_freed = total_freed.saturating_add(empty_recycle_bin_best_effort());

        let prefetch_root = Path::new(WINDOWS_PREFETCH_DIR);
        info!("deep purge start (root-and-recreate): {}", prefetch_root.display());
        total_freed = total_freed.saturating_add(purge_root_and_recreate(prefetch_root));
    }

    if config.gpu {
        let mut targets = vec![
            PathBuf::from(NVIDIA_INSTALLER_DIR),
            PathBuf::from(AMD_INSTALLER_DIR),
        ];
        if let Some(local) = &local_app_data {
            targets.push(local.join("Microsoft").join("DirectX Shader Cache"));
            targets.push(local.join("D3DSCache"));
            targets.push(local.join("NVIDIA").join("DXCache"));
            targets.push(local.join("NVIDIA").join("GLCache"));
            targets.push(local.join("AMD").join("DxCache"));
            targets.push(local.join("AMD").join("GLCache"));
        } else {
            warn!("deep purge skipped GPU LOCALAPPDATA targets: LOCALAPPDATA is unavailable");
        }
        for path in &targets {
            info!("deep purge start: {}", path.display());
            total_freed = total_freed.saturating_add(purge_children_only(path));
        }
    }

    if config.browsers {
        if let Some(local) = &local_app_data {
            let targets = vec![
                local
                    .join("Google")
                    .join("Chrome")
                    .join("User Data")
                    .join("Default")
                    .join("Cache")
                    .join("Cache_Data"),
                local
                    .join("Microsoft")
                    .join("Edge")
                    .join("User Data")
                    .join("Default")
                    .join("Cache")
                    .join("Cache_Data"),
                local
                    .join("BraveSoftware")
                    .join("Brave-Browser")
                    .join("User Data")
                    .join("Default")
                    .join("Cache")
                    .join("Cache_Data"),
            ];
            for path in &targets {
                info!("deep purge start: {}", path.display());
                total_freed = total_freed.saturating_add(purge_children_only(path));
            }
        } else {
            warn!("deep purge skipped browser cache targets: LOCALAPPDATA is unavailable");
        }
    }

    if config.apps {
        let mut targets = vec![PathBuf::from(PROGRAMDATA_BATTLENET_CACHE_DIR)];
        if let Some(local) = &local_app_data {
            targets.push(local.join("Spotify").join("Storage"));
            targets.push(local.join("Steam").join("htmlcache"));
            targets.push(
                local
                    .join("EpicGamesLauncher")
                    .join("Saved")
                    .join("webcache"),
            );
            targets.push(
                local
                    .join("Electronic Arts")
                    .join("EA Desktop")
                    .join("Cache"),
            );
            targets.push(
                local
                    .join("Riot Games")
                    .join("Riot Client")
                    .join("Data")
                    .join("Cache"),
            );
        } else {
            warn!("deep purge skipped app LOCALAPPDATA targets: LOCALAPPDATA is unavailable");
        }
        if let Some(app) = &app_data {
            targets.push(app.join("discord").join("Cache").join("Cache_Data"));
            targets.push(app.join("discord").join("Code Cache"));
            targets.push(app.join("Code").join("Cache"));
            targets.push(app.join("Code").join("CachedData"));
            targets.push(
                app.join("Telegram Desktop")
                    .join("tdata")
                    .join("user_data")
                    .join("cache"),
            );
            targets.push(
                app.join("Telegram Desktop")
                    .join("tdata")
                    .join("user_data")
                    .join("media_cache"),
            );
        } else {
            warn!("deep purge skipped app APPDATA targets: APPDATA is unavailable");
        }
        for path in &targets {
            info!("deep purge start: {}", path.display());
            total_freed = total_freed.saturating_add(purge_children_only(path));
        }
    }

    if config.dev {
        let mut targets = Vec::new();
        if let Some(local) = &local_app_data {
            targets.push(local.join("npm-cache"));
            targets.push(local.join("pip").join("cache"));
            targets.push(local.join("NuGet").join("v3-cache"));
            targets.push(local.join("go-build"));
            targets.push(local.join("Yarn").join("Cache"));
            targets.push(local.join("Composer"));
        } else {
            warn!("deep purge skipped dev LOCALAPPDATA targets: LOCALAPPDATA is unavailable");
        }
        if let Some(user) = &user_profile {
            targets.push(user.join(".npm"));
            targets.push(user.join(".cargo").join("registry").join("cache"));
            targets.push(user.join(".gradle").join("caches"));
        } else {
            warn!("deep purge skipped dev USERPROFILE targets: USERPROFILE is unavailable");
        }
        for path in &targets {
            info!("deep purge start: {}", path.display());
            total_freed = total_freed.saturating_add(purge_children_only(path));
        }
    }

    Ok(total_freed)
}

#[cfg(test)]
mod tests {
    use super::{purge_children_only, purge_root_and_recreate};
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "optimus_test_{}_{}_{}",
            prefix,
            std::process::id(),
            timestamp
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn write_file(path: &PathBuf, bytes: usize) {
        let mut file = fs::File::create(path).expect("create test file");
        let content = vec![b'a'; bytes];
        file.write_all(&content).expect("write test file");
    }

    #[test]
    fn children_only_purge_removes_nested_content_but_keeps_root() {
        let root = unique_temp_dir("children_only");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("create nested dir");
        let file_path = nested.join("payload.bin");
        write_file(&file_path, 64);

        let freed = purge_children_only(&root);
        assert!(freed >= 64);
        assert!(root.exists());
        assert!(!file_path.exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn root_and_recreate_purge_recreates_target_directory() {
        let root = unique_temp_dir("root_recreate");
        let nested = root.join("prefetch_like");
        fs::create_dir_all(&nested).expect("create nested dir");
        let file_path = nested.join("trace.pf");
        write_file(&file_path, 128);

        let freed = purge_root_and_recreate(&root);
        assert!(freed >= 128);
        assert!(root.exists());
        let entries = fs::read_dir(&root).expect("read recreated root");
        assert_eq!(entries.count(), 0);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_directory_returns_zero_without_error() {
        let missing = std::env::temp_dir().join(format!(
            "optimus_test_missing_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        if missing.exists() {
            let _ = fs::remove_dir_all(&missing);
        }
        assert_eq!(purge_children_only(&missing), 0);
    }

    #[test]
    fn purge_continues_when_one_file_cannot_be_deleted() {
        let root = unique_temp_dir("partial_failure");
        let deletable = root.join("deletable.bin");
        write_file(&deletable, 32);

        let locked = root.join("readonly.bin");
        write_file(&locked, 32);
        let mut perms = fs::metadata(&locked)
            .expect("metadata readonly")
            .permissions();
        perms.set_readonly(true);
        fs::set_permissions(&locked, perms).expect("set readonly permissions");

        let freed = purge_children_only(&root);
        assert!(freed >= 32);
        assert!(!deletable.exists());

        if let Ok(metadata) = fs::metadata(&locked) {
            let mut reset = metadata.permissions();
            reset.set_readonly(false);
            let _ = fs::set_permissions(&locked, reset);
        }
        let _ = fs::remove_dir_all(&root);
    }
}
