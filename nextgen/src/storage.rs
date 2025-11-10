use std::{any::Any, sync::Mutex};

#[cfg(any(test, not(target_os = "espidf")))]
use std::collections::HashMap;

use anyhow::{anyhow, Result};
#[cfg(target_os = "espidf")]
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::{
    nvs::{EspNvs, NvsDefault},
    sys::{EspError, ESP_ERR_NVS_NOT_FOUND},
};
use once_cell::sync::Lazy;

use crate::{
    auth::Credentials,
    util::{hex_decode, hex_encode},
};

const NVS_NAMESPACE: &str = "controller";
const NVS_KEY_USERNAME: &str = "user";
const NVS_KEY_PASSWORD_HASH: &str = "pwd_hash";
const NVS_KEY_SALT: &str = "pwd_salt";
const NVS_KEY_PROVISIONED: &str = "prov";
const NVS_KEY_WIFI_SSID: &str = "wifi_ssid";
const NVS_KEY_WIFI_PASSWORD: &str = "wifi_pwd";
const MAX_NVS_STR_LEN: usize = 128;

#[cfg(target_os = "espidf")]
pub static NVS_PARTITION: Lazy<EspDefaultNvsPartition> =
    Lazy::new(|| EspDefaultNvsPartition::take().expect("Failed to take default NVS partition"));

type DynNvs = dyn NvsAccess + Send;

#[cfg(all(target_os = "espidf", not(test)))]
static NVS: Lazy<Mutex<Box<DynNvs>>> = Lazy::new(|| {
    let partition = NVS_PARTITION.clone();
    let nvs = EspNvs::new(partition, NVS_NAMESPACE, true).expect("Failed to open NVS namespace");
    Mutex::new(Box::new(nvs) as Box<DynNvs>)
});

#[cfg(any(test, not(target_os = "espidf")))]
static NVS: Lazy<Mutex<Box<DynNvs>>> =
    Lazy::new(|| Mutex::new(Box::new(MockNvs::default()) as Box<DynNvs>));

trait NvsAccess: Send + Any {
    #[cfg_attr(not(test), allow(dead_code))]
    fn as_any(&mut self) -> &mut dyn Any;
    fn set_str(&mut self, key: &str, value: &str) -> Result<(), EspError>;
    fn get_str(&mut self, key: &str) -> Result<Option<String>, EspError>;
    fn set_u8(&mut self, key: &str, value: u8) -> Result<(), EspError>;
    fn get_u8(&mut self, key: &str) -> Result<Option<u8>, EspError>;
    fn remove(&mut self, key: &str) -> Result<(), EspError>;
    fn commit(&mut self) -> Result<(), EspError>;
}

impl NvsAccess for EspNvs<NvsDefault> {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn set_str(&mut self, key: &str, value: &str) -> Result<(), EspError> {
        Self::set_str(self, key, value)
    }

    fn get_str(&mut self, key: &str) -> Result<Option<String>, EspError> {
        let mut buffer = [0u8; MAX_NVS_STR_LEN];
        match Self::get_str(self, key, &mut buffer) {
            Ok(Some(value)) => Ok(Some(value.to_owned())),
            Ok(None) => Ok(None),
            Err(err) if err.code() == ESP_ERR_NVS_NOT_FOUND => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn set_u8(&mut self, key: &str, value: u8) -> Result<(), EspError> {
        Self::set_u8(self, key, value)
    }

    fn get_u8(&mut self, key: &str) -> Result<Option<u8>, EspError> {
        match Self::get_u8(self, key) {
            Ok(value) => Ok(value),
            Err(err) if err.code() == ESP_ERR_NVS_NOT_FOUND => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn remove(&mut self, key: &str) -> Result<(), EspError> {
        Self::remove(self, key).map(|_| ())
    }

    fn commit(&mut self) -> Result<(), EspError> {
        Ok(())
    }
}

#[cfg(any(test, not(target_os = "espidf")))]
#[derive(Default)]
struct MockNvs {
    strings: HashMap<String, String>,
    bytes: HashMap<String, u8>,
}

#[cfg(any(test, not(target_os = "espidf")))]
impl NvsAccess for MockNvs {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn set_str(&mut self, key: &str, value: &str) -> Result<(), EspError> {
        self.strings.insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn get_str(&mut self, key: &str) -> Result<Option<String>, EspError> {
        Ok(self.strings.get(key).cloned())
    }

    fn set_u8(&mut self, key: &str, value: u8) -> Result<(), EspError> {
        self.bytes.insert(key.to_owned(), value);
        Ok(())
    }

    fn get_u8(&mut self, key: &str) -> Result<Option<u8>, EspError> {
        Ok(self.bytes.get(key).copied())
    }

    fn remove(&mut self, key: &str) -> Result<(), EspError> {
        self.strings.remove(key);
        self.bytes.remove(key);
        Ok(())
    }

    fn commit(&mut self) -> Result<(), EspError> {
        Ok(())
    }
}

pub struct PersistentState {
    pub credentials: Credentials,
    pub provisioned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiCredentials {
    pub ssid: String,
    pub password: Option<String>,
}

pub fn load_persistent_state() -> Result<Option<PersistentState>> {
    let stored_values = with_nvs(|nvs| {
        let provisioned = nvs
            .get_u8(NVS_KEY_PROVISIONED)?
            .is_some_and(|value| value != 0);
        if !provisioned {
            return Ok(None);
        }

        let Some(username) = nvs.get_str(NVS_KEY_USERNAME)? else {
            return Ok(None);
        };
        let Some(password_hash) = nvs.get_str(NVS_KEY_PASSWORD_HASH)? else {
            return Ok(None);
        };
        let Some(salt_hex) = nvs.get_str(NVS_KEY_SALT)? else {
            return Ok(None);
        };

        Ok(Some((provisioned, username, password_hash, salt_hex)))
    })?;

    let Some((provisioned, username, password_hash, salt_hex)) = stored_values else {
        return Ok(None);
    };

    let salt_vec = hex_decode(&salt_hex).map_err(|err| anyhow!("Invalid salt encoding: {err}"))?;
    if salt_vec.len() != crate::SALT_LEN {
        return Err(anyhow!("Invalid salt length stored in NVS"));
    }
    let mut salt = [0u8; crate::SALT_LEN];
    salt.copy_from_slice(&salt_vec);

    Ok(Some(PersistentState {
        credentials: Credentials::from_persisted(username, password_hash, salt),
        provisioned,
    }))
}

pub fn persist_credentials_state(credentials: &Credentials, provisioned: bool) -> Result<()> {
    with_nvs(|nvs| {
        nvs.set_str(NVS_KEY_USERNAME, &credentials.username)?;
        nvs.set_str(NVS_KEY_PASSWORD_HASH, &credentials.password_hash)?;
        let salt_hex = hex_encode(&credentials.salt);
        nvs.set_str(NVS_KEY_SALT, &salt_hex)?;
        nvs.set_u8(NVS_KEY_PROVISIONED, u8::from(provisioned))?;
        nvs.commit()
    })?;
    Ok(())
}

pub fn clear_persistent_credentials() -> Result<()> {
    with_nvs(|nvs| {
        let _ = nvs.remove(NVS_KEY_USERNAME);
        let _ = nvs.remove(NVS_KEY_PASSWORD_HASH);
        let _ = nvs.remove(NVS_KEY_SALT);
        nvs.set_u8(NVS_KEY_PROVISIONED, 0)?;
        nvs.commit()
    })?;
    Ok(())
}

pub fn load_wifi_credentials() -> Result<Option<WifiCredentials>> {
    let stored = with_nvs(|nvs| {
        let ssid = match nvs.get_str(NVS_KEY_WIFI_SSID)? {
            Some(value) if !value.is_empty() => value,
            _ => return Ok(None),
        };
        let password = nvs
            .get_str(NVS_KEY_WIFI_PASSWORD)?
            .filter(|value| !value.is_empty());
        Ok(Some((ssid, password)))
    })?;

    Ok(stored.map(|(ssid, password)| WifiCredentials { ssid, password }))
}

pub fn persist_wifi_credentials(credentials: &WifiCredentials) -> Result<()> {
    with_nvs(|nvs| {
        nvs.set_str(NVS_KEY_WIFI_SSID, &credentials.ssid)?;
        let password_value = credentials.password.as_deref().unwrap_or("");
        nvs.set_str(NVS_KEY_WIFI_PASSWORD, password_value)?;
        nvs.commit()
    })?;
    Ok(())
}

pub fn clear_wifi_credentials() -> Result<()> {
    with_nvs(|nvs| {
        let _ = nvs.remove(NVS_KEY_WIFI_SSID);
        let _ = nvs.remove(NVS_KEY_WIFI_PASSWORD);
        nvs.commit()
    })?;
    Ok(())
}

fn with_nvs<F, R>(operation: F) -> Result<R>
where
    F: FnOnce(&mut dyn NvsAccess) -> Result<R, EspError>,
{
    let mut guard = NVS.lock().unwrap();
    operation(&mut **guard).map_err(Into::into)
}

#[cfg(any(test, not(target_os = "espidf")))]
impl MockNvs {
    fn clear(&mut self) {
        self.strings.clear();
        self.bytes.clear();
    }
}

#[cfg(test)]
pub fn reset_mock_nvs() {
    let mut guard = NVS.lock().unwrap();
    let store = guard.as_mut();
    let mock = store
        .as_any()
        .downcast_mut::<MockNvs>()
        .expect("Expected MockNvs in test environment");
    mock.clear();
}
