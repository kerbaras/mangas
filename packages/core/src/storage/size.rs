/// "0 B", "512 B", "1.5 KB", "12.3 MB", "2.04 GB" (decimal units, like the OS file managers).
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    match unit {
        0 => format!("{bytes} B"),
        _ if value >= 100.0 => format!("{value:.0} {}", UNITS[unit]),
        _ if value >= 10.0 => format!("{value:.1} {}", UNITS[unit]),
        _ => format!("{value:.2} {}", UNITS[unit]),
    }
}

#[cfg(test)]
mod tests {
    use super::human_size;

    #[test]
    fn picks_a_unit_and_keeps_three_significant_digits() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(999), "999 B");
        assert_eq!(human_size(1_500), "1.50 KB");
        assert_eq!(human_size(12_340_000), "12.3 MB");
        assert_eq!(human_size(204_000_000_000), "204 GB");
    }
}
