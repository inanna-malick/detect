/// uninhabited type, used to signify that something does not exist
/// provided typeclass instances never invoked but provided for
/// convenience
#[derive(Debug, Clone)]
pub enum Done {}

impl std::fmt::Display for Done {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unreachable!()
    }
}

/// Parse size values like "1mb", "100kb", etc. into bytes
///
/// Supports units: b, kb, mb, gb, tb (case-insensitive)
///
/// # Examples
/// ```
/// use detect::util::parse_size;
/// assert_eq!(parse_size("10mb").unwrap(), 10 * 1024 * 1024);
/// assert_eq!(parse_size("500KB").unwrap(), 500 * 1024);
/// ```
pub fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim().to_lowercase();

    // Find where the unit starts
    let mut unit_start = 0;
    for (i, ch) in s.char_indices() {
        if !ch.is_ascii_digit() && ch != '.' {
            unit_start = i;
            break;
        }
    }

    if unit_start == 0 {
        return Err(format!(
            "Invalid size '{s}': expected format like '10mb', '500kb'"
        ));
    }

    let number_str = &s[..unit_start];
    let unit_str = &s[unit_start..];

    let number: f64 = number_str
        .parse()
        .map_err(|_| format!("Invalid size '{s}': cannot parse numeric value '{number_str}'"))?;

    let multiplier = match unit_str {
        "b" | "byte" | "bytes" => 1.0,
        "k" | "kb" | "kilobyte" | "kilobytes" => 1024.0,
        "m" | "mb" | "megabyte" | "megabytes" => 1024.0 * 1024.0,
        "g" | "gb" | "gigabyte" | "gigabytes" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" | "terabyte" | "terabytes" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => {
            return Err(format!(
                "Invalid size '{s}': unknown unit '{unit_str}' (expected: b, kb, mb, gb, tb)"
            ))
        }
    };

    Ok((number * multiplier) as u64)
}
