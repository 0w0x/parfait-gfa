# parfait-gfa
a gfa v1 and v2 parser with validation and error reporting. originally built for the parfait gfa visualisation tool, but can be used standalone as an executable or crate.

currently a work in progress; the api is not stable.
please open issues for any features you would like to see.

## usage (cli)
prints any errors and shows file stats
```bash
parfait-gfa path/to/file.gfa
``` 

## example (crate)
```rust
use parfait_gfa::gfa::{GfaParser, ParseOptions, GFAVersion};
use parfait_gfa::optional_field::OptionalFieldValue;

let mut gfa = GfaParser::new();

// parse a gfa file
let result = gfa.parse("path/to/file.gfa", &ParseOptions::default());

match result {
  Ok(_) => println!("Parsed successfully"),
  Err(errors) => println!("Failed to parse file"),
}

// add an integer tag "ab" with value 12345 to all segments
for segment in gfa.segments_mut() {
    segment.tags.add_tag("ab", OptionalFieldValue::Int(12345));
}

// write the modified GFA to a new file
let _ = gfa.write_to_file("file_with_ab_tags.gfa", GFAVersion::V2);
```

## missing features
- groups cannot be derived into paths (they are still are parsed/validated)
- jump connections in walks are ignored, any valid link/jump is accepted
- optional field tags that use JSON are parsed as strings
- path/walk parsing isn't very efficient and may be slow on large files
- error messages are missing context in most cases

## licence
MIT