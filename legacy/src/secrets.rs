use std::collections::HashMap;

/// Get the set of secret SSIDs and passwords
#[must_use]
pub fn get() -> HashMap<&'static str, &'static str> {
    HashMap::from([("SSID1", "PASSWORD1"), ("SSID2", "PASSWORD2")])
}
