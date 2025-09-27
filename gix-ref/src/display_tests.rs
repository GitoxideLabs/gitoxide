//! Tests for Display implementations

#[cfg(test)]
mod display_tests {
    use crate::{FullName, PartialName};
    use std::convert::TryFrom;

    #[test]
    fn test_full_name_display() {
        let full_name = FullName::try_from("refs/heads/main").unwrap();
        assert_eq!(format!("{}", full_name), "refs/heads/main");
        assert_eq!(full_name.to_string(), "refs/heads/main");
    }

    #[test]
    fn test_full_name_ref_display() {
        let full_name = FullName::try_from("refs/heads/main").unwrap();
        let full_name_ref = full_name.as_ref();
        assert_eq!(format!("{}", full_name_ref), "refs/heads/main");
        assert_eq!(full_name_ref.to_string(), "refs/heads/main");
    }

    #[test]
    fn test_partial_name_display() {
        let partial_name = PartialName::try_from("heads/main").unwrap();
        assert_eq!(format!("{}", partial_name), "heads/main");
        assert_eq!(partial_name.to_string(), "heads/main");
    }

    #[test]
    fn test_partial_name_ref_display() {
        let partial_name = PartialName::try_from("heads/main").unwrap();
        let partial_name_ref = partial_name.as_ref();
        assert_eq!(format!("{}", partial_name_ref), "heads/main");
        assert_eq!(partial_name_ref.to_string(), "heads/main");
    }

    #[test]
    fn test_display_with_various_ref_types() {
        // Test various types of refs
        let refs = vec![
            "refs/heads/main",
            "refs/remotes/origin/main", 
            "refs/tags/v1.0.0",
            "HEAD",
        ];

        for ref_name in refs {
            let full_name = FullName::try_from(ref_name).unwrap();
            let full_name_ref = full_name.as_ref();
            
            assert_eq!(format!("{}", full_name), ref_name);
            assert_eq!(format!("{}", full_name_ref), ref_name);
        }
    }

    #[test]
    fn test_display_with_partial_names() {
        let partial_names = vec![
            "main",
            "heads/main",
            "remotes/origin/main",
            "tags/v1.0.0",
        ];

        for partial_name_str in partial_names {
            let partial_name = PartialName::try_from(partial_name_str).unwrap();
            let partial_name_ref = partial_name.as_ref();
            
            assert_eq!(format!("{}", partial_name), partial_name_str);
            assert_eq!(format!("{}", partial_name_ref), partial_name_str);
        }
    }
}