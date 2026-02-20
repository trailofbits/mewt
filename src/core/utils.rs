use std::str::FromStr;

/// Parse a comma-separated string into a Vec of values
/// Each value is trimmed and parsed using FromStr
/// Empty values are filtered out
/// Returns None if no valid values found
pub fn parse_csv<T>(csv_str: Option<&str>) -> Option<Vec<T>>
where
    T: FromStr,
{
    csv_str.map(|s| {
        s.split(',')
            .filter_map(|part| {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    T::from_str(trimmed).ok()
                }
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_none() {
        let result: Option<Vec<String>> = parse_csv(None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_csv_empty() {
        let result: Option<Vec<String>> = parse_csv(Some(""));
        assert_eq!(result, Some(vec![]));
    }

    #[test]
    fn test_parse_csv_single() {
        let result = parse_csv::<String>(Some("hello"));
        assert_eq!(result, Some(vec!["hello".to_string()]));
    }

    #[test]
    fn test_parse_csv_multiple() {
        let result = parse_csv::<String>(Some("one,two,three"));
        assert_eq!(
            result,
            Some(vec![
                "one".to_string(),
                "two".to_string(),
                "three".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_csv_with_spaces() {
        let result = parse_csv::<String>(Some(" one , two , three "));
        assert_eq!(
            result,
            Some(vec![
                "one".to_string(),
                "two".to_string(),
                "three".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_csv_with_empty_values() {
        let result = parse_csv::<String>(Some("one,,two,,,three"));
        assert_eq!(
            result,
            Some(vec![
                "one".to_string(),
                "two".to_string(),
                "three".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_csv_integers() {
        let result = parse_csv::<i32>(Some("1,2,3"));
        assert_eq!(result, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_parse_csv_integers_with_invalid() {
        let result = parse_csv::<i32>(Some("1,abc,3"));
        assert_eq!(result, Some(vec![1, 3]));
    }
}
