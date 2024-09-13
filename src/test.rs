use crate::diff::{DeepDiff, Diff, Value};
use serde_json::Value as JsonValue;
use std::fs;

#[cfg(test)]
mod tests {
    use std::time::Instant;
    use serde_json::json;
    use crate::diffs_to_json;
    use super::*;

    fn load_json(name: &str) -> JsonValue {
        let content = fs::read_to_string(format!("test_cases/{}.json", name))
            .expect(&format!("Failed to read file: {}.json", name));
        serde_json::from_str(&content).expect(&format!("Failed to parse JSON from file: {}.json", name))
    }

    #[test]
    fn test_simple_comparison() {
        let json1 = load_json("simple_1");
        let json2 = load_json("simple_2");

        let deep_diff = DeepDiff::new();
        let diffs = deep_diff.compare_json(&json1, &json2);

        assert_eq!(diffs.len(), 3);
        assert!(diffs.contains(&Diff::Changed("b".to_string(), Value::String("hello".to_string()), Value::String("world".to_string()))));
        assert!(diffs.contains(&Diff::Changed("c".to_string(), Value::Bool(true), Value::Bool(false))));
        assert!(diffs.contains(&Diff::Added("d[3]".to_string(), Value::Int(4))));
    }

    #[test]
    fn test_nested_comparison() {
        let json1 = load_json("nested_1");
        let json2 = load_json("nested_2");

        let deep_diff = DeepDiff::new();
        let diffs = deep_diff.compare_json(&json1, &json2);

        assert_eq!(diffs.len(), 5);
        assert!(diffs.contains(&Diff::Changed("a.x".to_string(), Value::Int(1), Value::Int(2))));
        assert!(diffs.contains(&Diff::Changed("a.y.z".to_string(), Value::String("nested".to_string()), Value::String("deeply nested".to_string()))));
        assert!(diffs.contains(&Diff::Changed("b[0].age".to_string(), Value::Int(30), Value::Int(31))));
        assert!(diffs.contains(&Diff::Changed("b[1].age".to_string(), Value::Int(25), Value::Int(35))));
        assert!(diffs.contains(&Diff::Changed("b[1].name".to_string(), Value::String("Bob".to_string()), Value::String("Charlie".to_string()))));
    }

    #[test]
    fn test_array_order() {
        let json1 = load_json("array_order_1");
        let json2 = load_json("array_order_2");

        let deep_diff = DeepDiff::new().ignore_order(true);
        let diffs = deep_diff.compare_json(&json1, &json2);
        println!("{:?}", diffs);
        assert_eq!(diffs.len(), 1);
        assert!(diffs.contains(&Diff::Added("diff_numbers".to_string(), Value::Int(3))));
    }

    #[test]
    fn test_float_comparison() {
        let json1 = load_json("float_comparison_1");
        let json2 = load_json("float_comparison_2");

        let deep_diff = DeepDiff::new().float_tolerance(0.1, true);
        let diffs = deep_diff.compare_json(&json1, &json2);

        assert_eq!(diffs.len(), 2);
        assert!(diffs.contains(&Diff::Changed("c[1]".to_string(), Value::Float(2.71828), Value::Float(2.21))));
        assert!(diffs.contains(&Diff::Changed("d.x".to_string(), Value::Float(99.1), Value::Float(0.11))));
    }
    #[test]
    fn test_float_comparison_absolute() {
        let json1 = load_json("float_comparison_1");
        let json2 = load_json("float_comparison_2");

        let deep_diff = DeepDiff::new().float_tolerance(1.0, false);
        let diffs = deep_diff.compare_json(&json1, &json2);

        assert_eq!(diffs.len(), 1);
        assert!(diffs.contains(&Diff::Changed("d.x".to_string(), Value::Float(99.1), Value::Float(0.11))));
    }

    #[test]
    fn test_large_json_performance() {
        let size = 500_000;

        // Generate large JSON objects
        let mut obj1 = serde_json::Map::new();
        let mut obj2 = serde_json::Map::new();

        for i in 0..size {
            let key = format!("key{}", i);
            let value = json!({
                "id": i,
                "value": format!("This is a test string number {}", i),
                "nested": {
                    "a": i * 2,
                    "b": [i, i + 1, i + 2],
                }
            });
            obj1.insert(key.clone(), value.clone());
            obj2.insert(key.clone(), value);
        }

        // Introduce some differences
        obj2.insert("key0".to_string(), json!({"modified": true}));
        obj2.remove("key1");

        let json1 = JsonValue::Object(obj1);
        let json2 = JsonValue::Object(obj2);

        let diff_tool = DeepDiff::new();

        let start_time = Instant::now();
        let diffs = diff_tool.compare_json(&json1, &json2);
        let duration = start_time.elapsed();
        let json = diffs_to_json(&diffs);

        println!("Time taken: {:?}", duration);
        println!("Number of diffs: {}", diffs.len());
        println!("Diffs as JSON: {}", json);

        assert!(duration < std::time::Duration::from_secs(10));
        assert_eq!(diffs.len(), 5);
    }
}
