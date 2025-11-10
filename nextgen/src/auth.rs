use std::time::{SystemTime, Instant};

use pbkdf2::pbkdf2_hmac;
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::{util::hex_encode, PBKDF2_ITERATIONS, SALT_LEN, SESSION_IDLE_TIMEOUT, TOKEN_LEN};

#[derive(Clone)]
pub struct Credentials {
    pub username: String,
    pub password_hash: String,
    pub salt: [u8; SALT_LEN],
    token: Option<SessionToken>,
}

#[derive(Clone)]
struct SessionToken {
    value: String,
    last_seen: SystemTime,
}

impl SessionToken {
    const fn new(value: String, now: SystemTime) -> Self {
        Self {
            value,
            last_seen: now,
        }
    }

    fn is_expired(&self, now: SystemTime) -> bool {
        now
            .duration_since(self.last_seen)
            .is_ok_and(|elapsed| elapsed >= SESSION_IDLE_TIMEOUT)
    }

    fn touch(&mut self, now: SystemTime) {
        self.last_seen = now;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenValidation {
    Authorized,
    Expired,
    Invalid,
}

impl Default for Credentials {
    fn default() -> Self {
        Self::with_password("admin", "admin")
    }
}

#[allow(clippy::missing_const_for_fn)]
impl Credentials {
    pub fn with_password(username: &str, password: &str) -> Self {
        let salt = generate_salt();
        let password_hash = derive_password_hash(password, &salt);
        Self {
            username: username.into(),
            password_hash,
            salt,
            token: None,
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn from_persisted(
        username: String,
        password_hash: String,
        salt: [u8; SALT_LEN],
    ) -> Self {
        Self {
            username,
            password_hash,
            salt,
            token: None,
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn verify_password(&self, candidate: &str) -> bool {
        let candidate_hash = derive_password_hash(candidate, &self.salt);
        constant_time_equals(&candidate_hash, &self.password_hash)
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn set_password(&mut self, new_password: &str) {
        self.salt = generate_salt();
        self.password_hash = derive_password_hash(new_password, &self.salt);
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn issue_token(&mut self) -> String {
        let token_value = generate_token();
        let now = SystemTime::now();
        self.token = Some(SessionToken::new(token_value.clone(), now));
        token_value
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn invalidate_token(&mut self) {
        self.token = None;
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn validate_token(&mut self, candidate: &str, now: SystemTime) -> TokenValidation {
        match self.token {
            Some(ref mut session) => {
                if !constant_time_equals(&session.value, candidate) {
                    return TokenValidation::Invalid;
                }
                if session.is_expired(now) {
                    self.invalidate_token();
                    TokenValidation::Expired
                } else {
                    session.touch(now);
                    TokenValidation::Authorized
                }
            }
            None => TokenValidation::Invalid,
        }
    }
}

fn generate_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    salt
}

fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_LEN];
    OsRng.fill_bytes(&mut bytes);
    hex_encode(&bytes)
}

fn derive_password_hash(password: &str, salt: &[u8]) -> String {
    let start = Instant::now();
    let mut output = [0u8; 32];
    // NOTE: If watchdog timeouts persist even after feature gating, refactor into chunked background task.
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut output);
    let elapsed = start.elapsed();
    log::info!(
        "PBKDF2 derivation completed: iterations={}, ms={} (feature=weak-derivation={})",
        PBKDF2_ITERATIONS,
        elapsed.as_millis(),
        cfg!(feature = "weak-derivation")
    );
    hex_encode(&output)
}

fn constant_time_equals(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.as_bytes().ct_eq(right.as_bytes()).unwrap_u8() == 1
}

#[cfg(test)]
mod tests {
    #[test]
    fn token_lifecycle() {
        let mut credentials = super::Credentials::with_password("tester", "password123");
        let token_value = credentials.issue_token();

        let result = credentials.validate_token(&token_value, std::time::SystemTime::now());
        assert_eq!(result, super::TokenValidation::Authorized);

        credentials.invalidate_token();
        assert_eq!(
            credentials.validate_token(&token_value, std::time::SystemTime::now()),
            super::TokenValidation::Invalid
        );
    }
}
