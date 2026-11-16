//! Fuzzes RoboCheck's candidate checker. Candidate source is untrusted, so
//! parsing and validating any bytes must never panic: a bad candidate is an
//! error, not a crash.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(src) = std::str::from_utf8(data) {
        if let Ok(candidate) = robocheck::candidate::parse(src) {
            let _ = robocheck::candidate::validate(&candidate);
        }
    }
});
