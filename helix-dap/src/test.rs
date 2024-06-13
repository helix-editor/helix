#[cfg(test)]
mod tests {
    use crate::Module;

    #[test]
    fn test_deserialize_module_id_from_number() {
        let raw = r#"{"id": 0, "name": "Name"}"#;
        let module: Module = serde_json::from_str(raw).expect("Error!");
        assert_eq!(module.id, "0");
    }

    #[test]
    fn test_deserialize_module_id_from_string() {
        let raw = r#"{"id": "0", "name": "Name"}"#;
        let module: Module = serde_json::from_str(raw).expect("Error!");
        assert_eq!(module.id, "0");
    }
}
