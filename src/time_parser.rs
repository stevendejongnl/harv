use crate::error::{HarjiraError, Result};

/// Parse hours from either decimal format (e.g., "1.5") or colon format (e.g., "1:30")
///
/// # Examples
///
/// ```
/// use harjira::time_parser::parse_hours;
///
/// assert_eq!(parse_hours("1.5").unwrap(), 1.5);
/// assert_eq!(parse_hours("1:30").unwrap(), 1.5);
/// assert_eq!(parse_hours("0:45").unwrap(), 0.75);
/// ```
pub fn parse_hours(input: &str) -> Result<f64> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(HarjiraError::InvalidEntry(
            "Hours input cannot be empty".to_string(),
        ));
    }

    let hours = if trimmed.contains(':') {
        parse_colon_format(trimmed)?
    } else {
        parse_decimal(trimmed)?
    };

    // Validate range
    if hours <= 0.0 {
        return Err(HarjiraError::InvalidEntry(
            "Hours must be greater than 0".to_string(),
        ));
    }

    if hours > 24.0 {
        return Err(HarjiraError::InvalidEntry(
            "Hours cannot exceed 24".to_string(),
        ));
    }

    Ok(hours)
}

/// Parse decimal format (e.g., "1.5", "2", "0.75")
fn parse_decimal(input: &str) -> Result<f64> {
    input
        .parse::<f64>()
        .map_err(|_| HarjiraError::InvalidEntry(format!("Invalid hours format: '{}'", input)))
}

/// Parse colon format (e.g., "1:30", "0:45", "2:15")
fn parse_colon_format(input: &str) -> Result<f64> {
    let parts: Vec<&str> = input.split(':').collect();

    if parts.len() != 2 {
        return Err(HarjiraError::InvalidEntry(
            "Colon format must be HH:MM (e.g., 1:30)".to_string(),
        ));
    }

    let hours_str = parts[0].trim();
    let minutes_str = parts[1].trim();

    // Parse hours
    let hours = hours_str.parse::<u32>().map_err(|_| {
        HarjiraError::InvalidEntry(format!("Invalid hours value: '{}'", hours_str))
    })?;

    // Parse minutes
    let minutes = minutes_str.parse::<u32>().map_err(|_| {
        HarjiraError::InvalidEntry(format!("Invalid minutes value: '{}'", minutes_str))
    })?;

    // Validate minutes range
    if minutes >= 60 {
        return Err(HarjiraError::InvalidEntry(format!(
            "Minutes must be between 0 and 59, got {}",
            minutes
        )));
    }

    // Calculate total hours
    let total_hours = hours as f64 + (minutes as f64 / 60.0);

    Ok(total_hours)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Decimal format tests
    #[test]
    fn test_parse_decimal_basic() {
        assert_eq!(parse_hours("1.5").unwrap(), 1.5);
        assert_eq!(parse_hours("2.25").unwrap(), 2.25);
        assert_eq!(parse_hours("0.75").unwrap(), 0.75);
    }

    #[test]
    fn test_parse_decimal_whole() {
        assert_eq!(parse_hours("1").unwrap(), 1.0);
        assert_eq!(parse_hours("2").unwrap(), 2.0);
        assert_eq!(parse_hours("8").unwrap(), 8.0);
    }

    #[test]
    fn test_parse_decimal_small() {
        assert_eq!(parse_hours("0.1").unwrap(), 0.1);
        assert_eq!(parse_hours("0.01").unwrap(), 0.01);
        assert_eq!(parse_hours("0.5").unwrap(), 0.5);
    }

    // Colon format tests
    #[test]
    fn test_parse_colon_basic() {
        assert_eq!(parse_hours("1:30").unwrap(), 1.5);
        assert_eq!(parse_hours("2:15").unwrap(), 2.25);
        assert_eq!(parse_hours("0:45").unwrap(), 0.75);
    }

    #[test]
    fn test_parse_colon_minutes_only() {
        assert_eq!(parse_hours("0:30").unwrap(), 0.5);
        assert_eq!(parse_hours("0:15").unwrap(), 0.25);
        assert_eq!(parse_hours("0:45").unwrap(), 0.75);
    }

    #[test]
    fn test_parse_colon_various() {
        assert_eq!(parse_hours("1:00").unwrap(), 1.0);
        assert_eq!(parse_hours("2:00").unwrap(), 2.0);
        assert_eq!(parse_hours("10:45").unwrap(), 10.75);
        assert_eq!(parse_hours("0:01").unwrap(), 1.0 / 60.0);
    }

    #[test]
    fn test_parse_colon_leading_zeros() {
        assert_eq!(parse_hours("01:30").unwrap(), 1.5);
        assert_eq!(parse_hours("00:45").unwrap(), 0.75);
        assert_eq!(parse_hours("02:15").unwrap(), 2.25);
    }

    // Whitespace handling
    #[test]
    fn test_whitespace_handling() {
        assert_eq!(parse_hours(" 1.5 ").unwrap(), 1.5);
        assert_eq!(parse_hours(" 1 : 30 ").unwrap(), 1.5);
        assert_eq!(parse_hours("  2.25  ").unwrap(), 2.25);
        assert_eq!(parse_hours("  0 : 45  ").unwrap(), 0.75);
    }

    // Validation boundaries
    #[test]
    fn test_validation_boundaries() {
        // Valid boundaries
        assert_eq!(parse_hours("24").unwrap(), 24.0);
        assert_eq!(parse_hours("0.01").unwrap(), 0.01);

        // Invalid: zero
        assert!(parse_hours("0").is_err());
        assert!(parse_hours("0.0").is_err());
        assert!(parse_hours("0:00").is_err());

        // Invalid: exceeds 24
        assert!(parse_hours("24.1").is_err());
        assert!(parse_hours("25").is_err());
        assert!(parse_hours("25:00").is_err());
    }

    #[test]
    fn test_negative_values() {
        assert!(parse_hours("-1").is_err());
        assert!(parse_hours("-1.5").is_err());
        assert!(parse_hours("-0.5").is_err());
    }

    // Invalid formats
    #[test]
    fn test_invalid_formats() {
        assert!(parse_hours("abc").is_err());
        assert!(parse_hours("one").is_err());
        assert!(parse_hours("1.2.3").is_err());
        assert!(parse_hours("").is_err());
        assert!(parse_hours("   ").is_err());
    }

    #[test]
    fn test_invalid_colon_formats() {
        // Minutes out of range
        assert!(parse_hours("1:60").is_err());
        assert!(parse_hours("1:90").is_err());
        assert!(parse_hours("0:99").is_err());

        // Missing parts
        assert!(parse_hours("1:").is_err());
        assert!(parse_hours(":30").is_err());
        assert!(parse_hours(":").is_err());

        // Too many parts
        assert!(parse_hours("1:30:00").is_err());

        // Invalid characters
        assert!(parse_hours("1:3a").is_err());
        assert!(parse_hours("a:30").is_err());
    }

    #[test]
    fn test_floating_point_in_colon_format() {
        // Floating point hours in colon format should fail
        // (colon format expects integer hours and minutes)
        assert!(parse_hours("1.5:30").is_err());
    }

    // Edge cases
    #[test]
    fn test_edge_cases() {
        // Just at boundary
        assert_eq!(parse_hours("23:59").unwrap(), 23.0 + 59.0 / 60.0);

        // Very small value
        assert_eq!(parse_hours("0:01").unwrap(), 1.0 / 60.0);

        // Large hours with minutes
        assert_eq!(parse_hours("20:30").unwrap(), 20.5);
    }
}
