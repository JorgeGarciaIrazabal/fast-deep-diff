use serde::Serialize;
use serde_json::Value as JsonValue;
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, PartialEq, Serialize)]
pub enum Diff {
    Added(String, Value),
    Removed(String, Value),
    Changed(String, Value, Value),
}

#[derive(Debug, Clone, Serialize)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Array(Vec<Value>),
    Dict(BTreeMap<String, Value>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => {
                if a.is_nan() && b.is_nan() {
                    true
                } else {
                    a == b
                }
            }
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Dict(a), Value::Dict(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Value::Int(i) => {
                state.write_u8(1);
                i.hash(state);
            }
            Value::Float(f) => {
                state.write_u8(2);
                f.to_bits().hash(state);
            }
            Value::String(s) => {
                state.write_u8(3);
                s.hash(state);
            }
            Value::Bool(b) => {
                state.write_u8(4);
                b.hash(state);
            }
            Value::Array(arr) => {
                state.write_u8(5);
                arr.hash(state);
            }
            Value::Dict(dict) => {
                state.write_u8(6);
                dict.hash(state);
            }
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => {
                a.partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Array(a), Value::Array(b)) => a.cmp(b),
            (Value::Dict(a), Value::Dict(b)) => a.cmp(b),
            (a_variant, b_variant) => a_variant.variant_order().cmp(&b_variant.variant_order()),
        }
    }
}

impl Value {
    fn variant_order(&self) -> u8 {
        match self {
            Value::Int(_) => 1,
            Value::Float(_) => 2,
            Value::String(_) => 3,
            Value::Bool(_) => 4,
            Value::Array(_) => 5,
            Value::Dict(_) => 6,
        }
    }
}

pub struct DeepDiff {
    ignore_order: bool,
    float_tolerance: Option<f64>,
    use_percent: bool,
}

impl DeepDiff {
    pub fn new() -> Self {
        DeepDiff {
            ignore_order: false,
            float_tolerance: None,
            use_percent: false,
        }
    }

    pub fn ignore_order(mut self, value: bool) -> Self {
        self.ignore_order = value;
        self
    }

    pub fn float_tolerance(mut self, value: f64, use_percent: bool) -> Self {
        self.float_tolerance = Some(value);
        self.use_percent = use_percent;
        self
    }

    pub fn compare(&self, v1: &Value, v2: &Value) -> Vec<Diff> {
        self.compare_recursive(v1, v2, String::new())
    }

    fn compare_recursive(&self, v1: &Value, v2: &Value, path: String) -> Vec<Diff> {
        match (v1, v2) {
            (Value::Dict(dict1), Value::Dict(dict2)) => self.compare_dicts(dict1, dict2, path),
            (Value::Array(arr1), Value::Array(arr2)) => self.compare_arrays(arr1, arr2, path),
            _ => {
                if self.values_equal(v1, v2) {
                    vec![]
                } else {
                    vec![Diff::Changed(path, v1.clone(), v2.clone())]
                }
            }
        }
    }

    fn compare_dicts(
        &self,
        dict1: &BTreeMap<String, Value>,
        dict2: &BTreeMap<String, Value>,
        path: String,
    ) -> Vec<Diff> {
        let mut diffs = Vec::new();

        for (key, value1) in dict1 {
            let new_path = if path.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", path, key)
            };
            match dict2.get(key) {
                Some(value2) => {
                    diffs.extend(self.compare_recursive(value1, value2, new_path));
                }
                None => diffs.push(Diff::Removed(new_path, value1.clone())),
            }
        }

        for (key, value2) in dict2 {
            if !dict1.contains_key(key) {
                let new_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };
                diffs.push(Diff::Added(new_path, value2.clone()));
            }
        }

        diffs
    }

    fn compare_arrays(&self, arr1: &[Value], arr2: &[Value], path: String) -> Vec<Diff> {
        if self.ignore_order {
            self.compare_arrays_unordered(arr1, arr2, path)
        } else {
            self.compare_arrays_ordered(arr1, arr2, path)
        }
    }

    fn compare_arrays_ordered(
        &self,
        arr1: &[Value],
        arr2: &[Value],
        path: String,
    ) -> Vec<Diff> {
        let mut diffs = Vec::new();
        let max_len = arr1.len().max(arr2.len());

        for i in 0..max_len {
            let new_path = if self.ignore_order { path.clone() } else { format!("{}[{}]", path, i) };
            match (arr1.get(i), arr2.get(i)) {
                (Some(v1), Some(v2)) => {
                    diffs.extend(self.compare_recursive(v1, v2, new_path));
                }
                (Some(v1), None) => diffs.push(Diff::Removed(new_path, v1.clone())),
                (None, Some(v2)) => diffs.push(Diff::Added(new_path, v2.clone())),
                (None, None) => unreachable!(),
            }
        }

        diffs
    }

    fn compare_arrays_unordered(
        &self,
        arr1: &[Value],
        arr2: &[Value],
        path: String,
    ) -> Vec<Diff> {
        let mut sorted1 = arr1.to_vec();
        let mut sorted2 = arr2.to_vec();

        sorted1.sort();
        sorted2.sort();

        self.compare_arrays_ordered(&sorted1, &sorted2, path)
    }

    fn values_equal(&self, v1: &Value, v2: &Value) -> bool {
        match (v1, v2) {
            (Value::Float(f1), Value::Float(f2)) => {
                if let Some(tolerance) = self.float_tolerance {
                    let diff = (f1 - f2).abs();
                    if self.use_percent {
                        let max = f1.abs().max(f2.abs());
                        diff / max <= tolerance
                    } else {
                        diff <= tolerance
                    }
                } else {
                    f1 == f2
                }
            }
            _ => v1 == v2,
        }
    }

    pub fn compare_json(&self, json1: &JsonValue, json2: &JsonValue) -> Vec<Diff> {
        let v1 = self.json_to_value(json1);
        let v2 = self.json_to_value(json2);
        self.compare(&v1, &v2)
    }

    fn json_to_value(&self, json: &JsonValue) -> Value {
        match json {
            JsonValue::Null => Value::String("null".to_string()),
            JsonValue::Bool(b) => Value::Bool(*b),
            JsonValue::Number(n) => {
                if n.is_i64() {
                    Value::Int(n.as_i64().unwrap())
                } else {
                    Value::Float(n.as_f64().unwrap())
                }
            }
            JsonValue::String(s) => Value::String(s.clone()),
            JsonValue::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.json_to_value(v)).collect())
            }
            JsonValue::Object(obj) => {
                let mut map = BTreeMap::new();
                for (k, v) in obj {
                    map.insert(k.clone(), self.json_to_value(v));
                }
                Value::Dict(map)
            }
        }
    }
}

pub fn diffs_to_json(diffs: &[Diff]) -> JsonValue {
    serde_json::to_value(diffs).unwrap()
}