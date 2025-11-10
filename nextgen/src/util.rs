use std::{
    fmt::Write,
    time::{SystemTime, UNIX_EPOCH},
};

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // write! never fails when writing to String
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

pub fn hex_decode(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

pub fn now_rfc3339() -> String {
    let system_now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let nanos = i128::from(system_now.as_secs()) * 1_000_000_000_i128
        + i128::from(system_now.subsec_nanos());
    let odt =
        OffsetDateTime::from_unix_timestamp_nanos(nanos).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    odt.format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}
