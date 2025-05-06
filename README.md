# Minparser
Simple parsing functions

This crate is a collection of objects and algorithms shared among different crates that needs to implement a parser. 
## Usage example

```rust
use minparser::prelude::*;
let view = ViewFile::new_default("My string   value");
let (step, mtc) = view.match_tool_string("My string").unwrap();
assert_eq!(mtc, "My string");
assert_eq!(step.get_view(), "   value"); 
let step = step.match_tool(minparser::utils::WhiteTool).unwrap();   // Use the WhiteTool tool to
assert_eq!(step.get_view(), "value");                               //match a sequence of whitespaces
assert!(step.match_tool('a').is_err()); // A missing match is an error
```

# Links

- [Crates.io](https://crates.io/crates/minparser).
