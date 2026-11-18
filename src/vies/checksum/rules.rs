//! National checksum arithmetic for already syntax-checked VAT identifiers.
//!
//! Adapted from eu-vat 0.1.11 (Apache-2.0), copyright 2026 Yayasan Pesantren
//! Cipansor and contributors. See THIRD-PARTY-LICENSE.txt for attribution.
//! Modified to use our existing syntax contract, omit unrelated registration
//! constraints and return None for unsupported variants. No normalization here.

pub(super) fn check(prefix: &str, n: &str) -> Option<bool> {
    let last = || digit(n.as_bytes()[n.len() - 1]);
    Some(match prefix {
        "AT" => last() == (6 - luhn(&n[1..8], false)).rem_euclid(10),
        "BE" => (number(&n[..8]) + number(&n[8..])) % 97 == 0,
        "BG" => bulgaria(n),
        "CY" => {
            let mapped = [1, 0, 5, 7, 9, 13, 15, 17, 19, 21];
            let sum: i64 = n[..8]
                .bytes()
                .enumerate()
                .map(|(i, b)| {
                    if i % 2 == 0 {
                        mapped[digit(b) as usize]
                    } else {
                        digit(b)
                    }
                })
                .sum();
            n.as_bytes()[8] == b'A' + (sum % 26) as u8
        }
        "CZ" => match n.len() {
            8 if !n.starts_with('9') => {
                let remainder = (11 - weighted(&n[..7], &[8, 7, 6, 5, 4, 3, 2])).rem_euclid(11);
                last() == if remainder == 0 { 1 } else { remainder % 10 }
            }
            9 if n.starts_with('6') => {
                let sum = weighted(&n[1..8], &[8, 7, 6, 5, 4, 3, 2]) % 11;
                last() == (8 - (10 - sum).rem_euclid(11)).rem_euclid(10)
            }
            10 => last() == number(&n[..9]) % 11 % 10,
            _ => return None,
        },
        "DE" | "HR" => mod_11_10(n),
        "DK" => weighted(n, &[2, 7, 6, 5, 4, 3, 2, 1]) % 11 == 0,
        "EE" => weighted(n, &[3, 7, 1, 3, 7, 1, 3, 7, 1]) % 10 == 0,
        "EL" => last() == weighted(&n[..8], &[256, 128, 64, 32, 16, 8, 4, 2]) % 11 % 10,
        "ES" => return spain(n),
        "FI" => weighted(n, &[7, 9, 10, 5, 8, 4, 2, 1]) % 11 == 0,
        "FR" if n.bytes().all(|b| b.is_ascii_digit()) => {
            // Only the VAT key: SIREN has its own rules and exceptions.
            number(&n[..2]) == (12 + 3 * (number(&n[2..]) % 97)) % 97
        }
        "HU" => weighted(n, &[9, 7, 3, 1, 9, 7, 3, 1]) % 10 == 0,
        "IE" => return ireland(n),
        "IT" => {
            let province = number(&n[7..10]);
            if !(1..=100).contains(&province) && !matches!(province, 120 | 121 | 888 | 999) {
                return None;
            }
            luhn(n, false) == 0
        }
        "LT" => {
            if n.as_bytes()[n.len() - 2] != b'1' {
                return None;
            }
            let body = &n[..n.len() - 1];
            let sum = |offset: usize| {
                body.bytes()
                    .enumerate()
                    .map(|(i, b)| digit(b) * (1 + (i + offset) % 9) as i64)
                    .sum::<i64>()
                    % 11
            };
            let first = sum(0);
            last() == if first == 10 { sum(2) % 10 } else { first }
        }
        "LU" => number(&n[6..]) == number(&n[..6]) % 89,
        "LV" if n.starts_with("32") => return None,
        "LV" if n.as_bytes()[0] > b'3' => {
            weighted(n, &[9, 1, 4, 8, 3, 10, 2, 5, 7, 6, 1]) % 11 == 3
        }
        "LV" => last() == (1 + weighted(&n[..10], &[10, 5, 8, 4, 2, 1, 6, 3, 7, 9])) % 11 % 10,
        "MT" => weighted(n, &[3, 4, 6, 7, 8, 9, 10, 1]) % 37 == 0,
        "NL" => {
            // Pre-2020 BSN-based identifiers OR the newer whole-ID modulo 97.
            weighted(&n[..9], &[9, 8, 7, 6, 5, 4, 3, 2, -1]).rem_euclid(11) == 0
                || mod_97_10(&format!("NL{n}"))
        }
        "PL" => weighted(n, &[6, 5, 7, 2, 3, 4, 5, 6, 7, -1]).rem_euclid(11) == 0,
        "PT" => last() == (11 - weighted(&n[..8], &[9, 8, 7, 6, 5, 4, 3, 2])).rem_euclid(11) % 10,
        "RO" => {
            let weights = [7, 5, 3, 2, 1, 7, 5, 3, 2];
            let body = &n[..n.len() - 1];
            last() == (10 * weighted(body, &weights[9 - body.len()..])) % 11 % 10
        }
        "SE" => luhn(&n[..10], false) == 0,
        "SI" => {
            let check = 11 - weighted(&n[..7], &[8, 7, 6, 5, 4, 3, 2]) % 11;
            last() == if check == 10 { 0 } else { check }
        }
        "SK" => {
            // Accept both company and historical personal-number checks.
            number(n) % 11 == 0 || last() == number(&n[..9]) % 11 % 10
        }
        "XI" if n.bytes().all(|b| b.is_ascii_digit()) => {
            let remainder = weighted(&n[..9], &[8, 7, 6, 5, 4, 3, 2, 10, 1]) % 97;
            remainder == 0 || (number(&n[..3]) >= 100 && matches!(remainder, 42 | 55))
        }
        _ => return None,
    })
}

fn digit(b: u8) -> i64 {
    i64::from(b - b'0')
}
fn number(n: &str) -> i64 {
    n.bytes().fold(0, |sum, b| sum * 10 + digit(b))
}
fn weighted(n: &str, weights: &[i64]) -> i64 {
    n.bytes()
        .zip(weights)
        .map(|(b, weight)| digit(b) * weight)
        .sum()
}

// `double_last` computes a payload's contribution before its check digit is added.
fn luhn(n: &str, double_last: bool) -> i64 {
    n.bytes()
        .rev()
        .enumerate()
        .map(|(i, b)| {
            let value = digit(b) * if (i % 2 == 1) ^ double_last { 2 } else { 1 };
            value / 10 + value % 10
        })
        .sum::<i64>()
        % 10
}

fn mod_11_10(n: &str) -> bool {
    n.bytes().fold(5, |state, b| {
        let carry = if state == 0 { 10 } else { state };
        ((carry * 2) % 11 + digit(b)) % 10
    }) == 1
}

fn mod_97_10(n: &str) -> bool {
    n.bytes().fold(0, |remainder, b| {
        let value = if b.is_ascii_digit() {
            digit(b)
        } else {
            i64::from(b - b'A') + 10
        };
        (remainder * if value < 10 { 10 } else { 100 } + value) % 97
    }) == 1
}

fn bulgaria(n: &str) -> bool {
    let last = digit(n.as_bytes()[n.len() - 1]);
    if n.len() == 9 {
        let first = weighted(&n[..8], &[1, 2, 3, 4, 5, 6, 7, 8]) % 11;
        last == if first == 10 {
            weighted(&n[..8], &[3, 4, 5, 6, 7, 8, 9, 10]) % 11 % 10
        } else {
            first
        }
    } else {
        // Personal, foreign-person, and other ten-digit registration schemes.
        last == weighted(&n[..9], &[2, 4, 8, 5, 10, 9, 7, 3, 6]) % 11 % 10
            || last == weighted(&n[..9], &[21, 19, 17, 13, 11, 9, 7, 3, 1]) % 10
            || last == (11 - weighted(&n[..9], &[4, 3, 2, 7, 6, 5, 4, 3, 2])).rem_euclid(11)
    }
}

fn spain(n: &str) -> Option<bool> {
    let first = n.as_bytes()[0];
    let last = n.as_bytes()[8];
    let letters = b"TRWAGMYFPDXBNJZSQVHLCKE";
    let value = match first {
        b'0'..=b'9' => number(&n[..8]),
        b'K' | b'L' | b'M' => number(&n[1..8]),
        b'X'..=b'Z' => i64::from(first - b'X') * 10_000_000 + number(&n[1..8]),
        _ if b"ABCDEFGHJNPQRSUVW".contains(&first) => {
            let check = (10 - luhn(&n[1..8], true)) % 10;
            return Some(last == b'0' + check as u8 || last == b"JABCDEFGHI"[check as usize]);
        }
        _ => return None,
    };
    Some(last == letters[(value % 23) as usize])
}

fn ireland(n: &str) -> Option<bool> {
    let alphabet = b"WABCDEFGHIJKLMNOPQRSTUV";
    let sum = if n[..7].bytes().all(|b| b.is_ascii_digit()) {
        let suffix = if n.len() == 9 {
            alphabet.iter().position(|b| *b == n.as_bytes()[8])? as i64
        } else {
            0
        };
        weighted(&n[..7], &[8, 7, 6, 5, 4, 3, 2]) + 9 * suffix
    } else {
        // Legacy second-position letter, '*' or '+': normalize only arithmetic.
        weighted(&n[2..7], &[7, 6, 5, 4, 3]) + 2 * digit(n.as_bytes()[0])
    };
    Some(n.as_bytes()[7] == alphabet[(sum % 23) as usize])
}
