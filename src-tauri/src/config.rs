use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::OnceCell;
use redb::{Database, ReadableDatabase as _, ReadableTable as _, TableDefinition};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::utils::paths::{data_db_path, ensure_data_dirs};

static CONFIG_DB: OnceCell<Database> = OnceCell::new();
const CONFIG_TABLE: TableDefinition<u8, &[u8]> = TableDefinition::new("app_config");
const MANIFEST_TABLE: TableDefinition<u8, &[u8]> = TableDefinition::new("app_manifest");
const ROOT_ROW_KEY: u8 = 0;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub mainland_acceleration: bool,
    #[serde(default)]
    pub github_proxy: String,
    #[serde(default)]
    pub proxy_url: String,
    #[serde(default)]
    pub proxy_port: String,
    #[serde(default)]
    pub proxy_username: String,
    #[serde(default)]
    pub proxy_password: String,
    #[serde(default)]
    pub pypi_mirror: String,
    #[serde(default)]
    pub nodejs_mirror: String,
    #[serde(default)]
    pub npm_registry: String,
    #[serde(default)]
    pub use_uv_for_deps: bool,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    #[serde(default = "default_true")]
    pub check_instance_update: bool,
    #[serde(default)]
    pub persist_instance_state: bool,
    #[serde(default)]
    pub ignore_external_path: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mainland_acceleration: false,
            github_proxy: String::new(),
            proxy_url: String::new(),
            proxy_port: String::new(),
            proxy_username: String::new(),
            proxy_password: String::new(),
            pypi_mirror: String::new(),
            nodejs_mirror: String::new(),
            npm_registry: String::new(),
            use_uv_for_deps: false,
            close_to_tray: true,
            check_instance_update: true,
            persist_instance_state: false,
            ignore_external_path: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppManifest {
    #[serde(default)]
    pub instances: HashMap<String, InstanceConfig>,
    #[serde(default)]
    pub installed_versions: Vec<InstalledVersion>,
    #[serde(default)]
    pub tracked_instances_snapshot: Vec<String>,
}

fn map_redb_error(err: impl std::fmt::Display) -> AppError {
    AppError::config(err.to_string())
}

fn open_config_db() -> Result<Database> {
    ensure_data_dirs()?;
    let db = Database::create(data_db_path()).map_err(map_redb_error)?;
    let write_txn = db.begin_write().map_err(map_redb_error)?;
    {
        let _ = write_txn.open_table(CONFIG_TABLE).map_err(map_redb_error)?;
        let _ = write_txn
            .open_table(MANIFEST_TABLE)
            .map_err(map_redb_error)?;
    }
    write_txn.commit().map_err(map_redb_error)?;
    Ok(db)
}

fn config_db() -> Result<&'static Database> {
    CONFIG_DB.get_or_try_init(open_config_db)
}

fn serialize_value<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| AppError::config(e.to_string()))
}

fn deserialize_value<T: DeserializeOwned>(raw: &[u8]) -> Result<T> {
    serde_json::from_slice(raw).map_err(|e| AppError::config(e.to_string()))
}

fn read_value<T: DeserializeOwned>(table_def: TableDefinition<u8, &[u8]>) -> Result<Option<T>> {
    let db = config_db()?;
    let read_txn = db.begin_read().map_err(map_redb_error)?;
    let table = read_txn.open_table(table_def).map_err(map_redb_error)?;
    let value = table.get(&ROOT_ROW_KEY).map_err(map_redb_error)?;

    if let Some(raw) = value {
        return deserialize_value::<T>(raw.value()).map(Some);
    }

    Ok(None)
}

fn load_or_init_value<T: DeserializeOwned + Serialize>(
    table_def: TableDefinition<u8, &[u8]>,
    default: T,
) -> Result<T> {
    if let Some(existing) = read_value(table_def)? {
        return Ok(existing);
    }

    let db = config_db()?;
    let write_txn = db.begin_write().map_err(map_redb_error)?;

    let existing = {
        let table = write_txn.open_table(table_def).map_err(map_redb_error)?;
        let value = table.get(&ROOT_ROW_KEY).map_err(map_redb_error)?;
        if let Some(raw) = value {
            Some(deserialize_value::<T>(raw.value())?)
        } else {
            None
        }
    };

    if let Some(value) = existing {
        return Ok(value);
    }

    let payload = serialize_value(&default)?;
    {
        let mut table = write_txn.open_table(table_def).map_err(map_redb_error)?;
        table
            .insert(&ROOT_ROW_KEY, payload.as_slice())
            .map_err(map_redb_error)?;
    }
    write_txn.commit().map_err(map_redb_error)?;

    Ok(default)
}

fn with_value_mut<T, F, R>(table_def: TableDefinition<u8, &[u8]>, default: T, f: F) -> Result<R>
where
    T: DeserializeOwned + Serialize,
    F: FnOnce(&mut T) -> Result<R>,
{
    let db = config_db()?;
    let write_txn = db.begin_write().map_err(map_redb_error)?;

    let mut value = {
        let table = write_txn.open_table(table_def).map_err(map_redb_error)?;
        let existing = table.get(&ROOT_ROW_KEY).map_err(map_redb_error)?;
        if let Some(raw) = existing {
            deserialize_value::<T>(raw.value())?
        } else {
            default
        }
    };

    let result = f(&mut value)?;
    let payload = serialize_value(&value)?;

    {
        let mut table = write_txn.open_table(table_def).map_err(map_redb_error)?;
        table
            .insert(&ROOT_ROW_KEY, payload.as_slice())
            .map_err(map_redb_error)?;
    }
    write_txn.commit().map_err(map_redb_error)?;

    Ok(result)
}

fn has_value(table_def: TableDefinition<u8, &[u8]>) -> Result<bool> {
    let db = config_db()?;
    let read_txn = db.begin_read().map_err(map_redb_error)?;
    let table = read_txn.open_table(table_def).map_err(map_redb_error)?;
    let value = table.get(&ROOT_ROW_KEY).map_err(map_redb_error)?;
    Ok(value.is_some())
}

/// Execute a transactional read-modify-write operation on app config.
pub fn with_config_mut<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&mut AppConfig) -> Result<T>,
{
    with_value_mut(CONFIG_TABLE, AppConfig::default(), f)
}

/// Execute a transactional read-modify-write operation on app manifest.
pub fn with_manifest_mut<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&mut AppManifest) -> Result<T>,
{
    with_value_mut(MANIFEST_TABLE, AppManifest::default(), f)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstalledVersion {
    pub version: String,
    pub zip_path: String,
}

/// Backup metadata stored in backup.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub created_at: String,
    pub instance_name: String,
    pub instance_id: String,
    pub version: String,
    #[serde(default)]
    pub includes_venv: bool,
    #[serde(default = "default_true")]
    pub includes_data: bool,
    #[serde(default)]
    pub arch_target: String,
    #[serde(default)]
    pub auto_generated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub filename: String,
    pub path: String,
    pub metadata: BackupMetadata,
    #[serde(default)]
    pub corrupted: bool,
    #[serde(default)]
    pub parse_error: Option<String>,
}

pub fn load_config() -> Result<Arc<AppConfig>> {
    let config = load_or_init_value(CONFIG_TABLE, AppConfig::default())?;
    Ok(Arc::new(config))
}

pub fn reload_config() -> Result<Arc<AppConfig>> {
    load_config()
}

pub fn load_manifest() -> Result<Arc<AppManifest>> {
    let manifest = load_or_init_value(MANIFEST_TABLE, AppManifest::default())?;
    Ok(Arc::new(manifest))
}

pub fn reload_manifest() -> Result<Arc<AppManifest>> {
    load_manifest()
}

pub(crate) fn has_config_record() -> Result<bool> {
    has_value(CONFIG_TABLE)
}

pub(crate) fn has_manifest_record() -> Result<bool> {
    has_value(MANIFEST_TABLE)
}
