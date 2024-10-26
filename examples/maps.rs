use std::collections::HashMap;

fn main() {
    let data: HashMap<String, HashMap<String, HashMap<String, i64>>> = HashMap::new();

    let value = data
        .get("hello")
        .and_then(|inner| inner.get("world"))
        .and_then(|inner| inner.get("age"))
        .unwrap_or(&0);

    println!("{:?}", value);
}
