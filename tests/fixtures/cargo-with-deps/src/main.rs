// Cargo project with external dependencies
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    name: String,
    age: u32,
}

fn main() {
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };

    let json = serde_json::to_string(&person).unwrap();
    println!("{}", json);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let person = Person {
            name: "Bob".to_string(),
            age: 25,
        };
        let json = serde_json::to_string(&person).unwrap();
        assert!(json.contains("Bob"));
    }
}
