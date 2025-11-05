use crate::errors::{ParseMessage, ParseMessageCode};
use std::{collections::HashMap, convert::TryFrom};

#[derive(Debug, Clone)]
pub enum OptionalFieldValue {
    Char(char),                            // A
    Int(i32),                              // i
    Float(f32),                            // f
    String(String),                        // Z
    Json(String),                          // J
    ByteArray(Vec<u8>),                    // H
    NumberArray(Vec<OptionalFieldNumber>), // B
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Char,        // 'A'
    Int,         // 'i'
    Float,       // 'f'
    String,      // 'Z'
    Json,        // 'J'
    ByteArray,   // 'H'
    NumberArray, // 'B'
}

impl TryFrom<&OptionalFieldValue> for char {
    type Error = ParseMessageCode;

    fn try_from(v: &OptionalFieldValue) -> Result<Self, Self::Error> {
        match v {
            OptionalFieldValue::Char(c) => Ok(*c),
            _ => Err(ParseMessageCode::OptionalFieldValueTypeMismatch),
        }
    }
}

impl TryFrom<&OptionalFieldValue> for i32 {
    type Error = ParseMessageCode;

    fn try_from(v: &OptionalFieldValue) -> Result<Self, Self::Error> {
        match v {
            OptionalFieldValue::Int(i) => Ok(*i),
            _ => Err(ParseMessageCode::OptionalFieldValueTypeMismatch),
        }
    }
}

impl TryFrom<&OptionalFieldValue> for f32 {
    type Error = ParseMessageCode;

    fn try_from(v: &OptionalFieldValue) -> Result<Self, Self::Error> {
        match v {
            OptionalFieldValue::Float(f) => Ok(*f),
            _ => Err(ParseMessageCode::OptionalFieldValueTypeMismatch),
        }
    }
}

impl TryFrom<&OptionalFieldValue> for String {
    type Error = ParseMessageCode;

    fn try_from(v: &OptionalFieldValue) -> Result<Self, Self::Error> {
        match v {
            OptionalFieldValue::String(s) => Ok(s.clone()),
            OptionalFieldValue::Json(j) => Ok(j.clone()),
            _ => Err(ParseMessageCode::OptionalFieldValueTypeMismatch),
        }
    }
}

impl TryFrom<&OptionalFieldValue> for Vec<u8> {
    type Error = ParseMessageCode;
    fn try_from(v: &OptionalFieldValue) -> Result<Self, Self::Error> {
        match v {
            OptionalFieldValue::ByteArray(b) => Ok(b.clone()),
            _ => Err(ParseMessageCode::OptionalFieldValueTypeMismatch),
        }
    }
}

impl TryFrom<&OptionalFieldValue> for Vec<OptionalFieldNumber> {
    type Error = ParseMessageCode;
    fn try_from(v: &OptionalFieldValue) -> Result<Self, Self::Error> {
        match v {
            OptionalFieldValue::NumberArray(arr) => Ok(arr.clone()),
            _ => Err(ParseMessageCode::OptionalFieldValueTypeMismatch),
        }
    }
}

impl TryFrom<char> for FieldType {
    type Error = ParseMessageCode;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            'A' => Ok(FieldType::Char),
            'i' => Ok(FieldType::Int),
            'f' => Ok(FieldType::Float),
            'Z' => Ok(FieldType::String),
            'J' => Ok(FieldType::Json),
            'H' => Ok(FieldType::ByteArray),
            'B' => Ok(FieldType::NumberArray),
            _ => Err(ParseMessageCode::InvalidOptionalFieldType),
        }
    }
}

impl OptionalFieldValue {
    pub fn get_field_type(&self) -> FieldType {
        match self {
            OptionalFieldValue::Char(_) => FieldType::Char,
            OptionalFieldValue::Int(_) => FieldType::Int,
            OptionalFieldValue::Float(_) => FieldType::Float,
            OptionalFieldValue::String(_) => FieldType::String,
            OptionalFieldValue::Json(_) => FieldType::Json,
            OptionalFieldValue::ByteArray(_) => FieldType::ByteArray,
            OptionalFieldValue::NumberArray(_) => FieldType::NumberArray,
        }
    }
}

impl std::fmt::Display for OptionalFieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptionalFieldValue::Char(c) => write!(f, "{c}"),
            OptionalFieldValue::Int(i) => write!(f, "{i}"),
            OptionalFieldValue::Float(float) => write!(f, "{float}"),
            OptionalFieldValue::String(s) => write!(f, "{s}"),
            OptionalFieldValue::Json(j) => write!(f, "{j}"),
            OptionalFieldValue::ByteArray(b) => write!(f, "{b:?}"),
            OptionalFieldValue::NumberArray(arr) => write!(f, "{arr:?}"),
        }
    }
}

impl FieldType {
    pub fn get_char(&self) -> char {
        match self {
            FieldType::Char => 'A',
            FieldType::Int => 'i',
            FieldType::Float => 'f',
            FieldType::String => 'Z',
            FieldType::Json => 'J',
            FieldType::ByteArray => 'H',
            FieldType::NumberArray => 'B',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptionalFieldNumber {
    Int8(i8),     // c
    UInt8(u8),    // C
    Int16(i16),   // s
    UInt16(u16),  // S
    Int32(i32),   // i
    UInt32(u32),  // I
    Float32(f32), // f
}

#[derive(Debug)]
pub struct OptionalField {
    pub tag: String,
    pub type_: FieldType,
    pub value: OptionalFieldValue,
}

struct ReservedField {
    type_: FieldType,
    allowed_records: &'static [&'static char],
}

fn get_reserved_field(tag: &str) -> Option<&'static ReservedField> {
    match tag {
        "VN" => Some(&ReservedField {
            type_: FieldType::String,
            allowed_records: &[&'H'],
        }),
        "TS" => Some(&ReservedField {
            type_: FieldType::Int,
            allowed_records: &[&'H'],
        }),
        "LN" => Some(&ReservedField {
            type_: FieldType::Int,
            allowed_records: &[&'S'],
        }),
        "RC" => Some(&ReservedField {
            type_: FieldType::Int,
            allowed_records: &[&'S', &'L', &'C'],
        }),
        "FC" => Some(&ReservedField {
            type_: FieldType::Int,
            allowed_records: &[&'S', &'L'],
        }),
        "KC" => Some(&ReservedField {
            type_: FieldType::Int,
            allowed_records: &[&'S', &'L'],
        }),
        "SH" => Some(&ReservedField {
            type_: FieldType::ByteArray,
            allowed_records: &[&'S'],
        }),
        "UR" => Some(&ReservedField {
            type_: FieldType::String,
            allowed_records: &[&'S'],
        }),
        "MQ" => Some(&ReservedField {
            type_: FieldType::Int,
            allowed_records: &[&'L'],
        }),
        "NM" => Some(&ReservedField {
            type_: FieldType::Int,
            allowed_records: &[&'L', &'C'],
        }),
        "ID" => Some(&ReservedField {
            type_: FieldType::String,
            allowed_records: &[&'L', &'C', &'J'],
        }),
        "SC" => Some(&ReservedField {
            type_: FieldType::Int,
            allowed_records: &[&'J'],
        }),
        _ => None,
    }
}

fn check_optional_field_tag_context(
    line: usize,
    record_type: &char,
    tag_type: FieldType,
    tag: &str,
) -> Result<(), ParseMessage> {
    if let Some(reserved) = get_reserved_field(tag) {
        if reserved.type_ != tag_type {
            return Err(ParseMessage {
                line,
                code: ParseMessageCode::InvalidOptionalFieldReservedTagType,
                offender: tag.to_string(),
            });
        }
        if !reserved.allowed_records.contains(&record_type) {
            return Err(ParseMessage {
                line,
                code: ParseMessageCode::UnexpectedReservedTagType,
                offender: tag.to_string(),
            });
        }
        Ok(())
    } else {
        Ok(())
    }
}

pub fn parse_optional_field_value(
    line: usize,
    ftype: FieldType,
    value: &str,
) -> (Option<OptionalFieldValue>, Vec<ParseMessage>) {
    let mut errors = Vec::new();

    if value.is_empty() {
        errors.push(ParseMessage {
            line,
            code: ParseMessageCode::OptionalFieldValueEmpty,
            offender: "".to_string(),
        });
        return (None, errors);
    }

    let result = match ftype {
        FieldType::Char => {
            if value.chars().count() == 1 {
                Some(OptionalFieldValue::Char(value.chars().next().unwrap()))
            } else {
                errors.push(ParseMessage {
                    line,
                    code: ParseMessageCode::OptionalFieldValueTypeMismatch,
                    offender: value.to_string(),
                });
                None
            }
        }
        FieldType::Int => match value.parse::<i32>() {
            Ok(v) => Some(OptionalFieldValue::Int(v)),
            Err(_) => {
                errors.push(ParseMessage {
                    line,
                    code: ParseMessageCode::OptionalFieldValueTypeMismatch,
                    offender: value.to_string(),
                });
                None
            }
        },
        FieldType::Float => match value.parse::<f32>() {
            Ok(v) => Some(OptionalFieldValue::Float(v)),
            Err(_) => {
                errors.push(ParseMessage {
                    line,
                    code: ParseMessageCode::OptionalFieldValueTypeMismatch,
                    offender: value.to_string(),
                });
                None
            }
        },
        FieldType::String => Some(OptionalFieldValue::String(value.to_string())),
        FieldType::Json => Some(OptionalFieldValue::Json(value.to_string())), // TODO: handle JSON?
        FieldType::ByteArray => Some(OptionalFieldValue::ByteArray(value.as_bytes().to_vec())),
        FieldType::NumberArray => {
            let mut nums = Vec::new();

            for chunk in value.split(',') {
                if let Ok(i) = chunk.parse::<i32>() {
                    nums.push(OptionalFieldNumber::Int32(i));
                } else if let Ok(f) = chunk.parse::<f32>() {
                    nums.push(OptionalFieldNumber::Float32(f));
                } else {
                    errors.push(ParseMessage {
                        line,
                        code: ParseMessageCode::OptionalFieldValueTypeMismatch,
                        offender: chunk.to_string(),
                    });
                }
            }

            Some(OptionalFieldValue::NumberArray(nums))
        }
    };

    (result, errors)
}

pub fn collect_optional_fields(
    line: usize,
    record_type: &str,
    fields: &[&str],
) -> (Vec<OptionalField>, Vec<ParseMessage>) {
    let mut optional_fields = Vec::new();
    let mut errors = Vec::new();
    let mut used_tags = Vec::new();

    let record_type_char = record_type.chars().next().unwrap_or(' ');

    // check for duplicate optional fields
    for field in fields {
        let (parsed_field, field_errors) = parse_optional_field(line, &record_type_char, field);
        if let Some(f) = parsed_field {
            if used_tags.contains(&f.tag) { // TODO: benchmark against a hashset, should be faster
                errors.push(ParseMessage {
                    line,
                    code: ParseMessageCode::DuplicateOptionalField,
                    offender: f.tag.clone(),
                });
            } else {
                used_tags.push(f.tag.clone());
                optional_fields.push(f);
            }
        }
        errors.extend(field_errors);
    }

    (optional_fields, errors)
}

fn parse_optional_field(
    line: usize,
    record_type: &char,
    field: &str,
) -> (Option<OptionalField>, Vec<ParseMessage>) {
    let mut errors = Vec::new();

    let mut it = field.splitn(3, ':');

    let tag = it.next().unwrap_or_default();
    let type_str = it.next().unwrap_or_default();
    let value = it.next().unwrap_or_default();

    if tag.is_empty() || type_str.is_empty() {
        errors.push(ParseMessage {
            line,
            code: ParseMessageCode::InvalidOptionalField,
            offender: field.to_string(),
        });
        return (None, errors);
    }

    // tag should be two characters long
    // tag should either be uppercase (reserved) or lowercase (user-defined); not both
    // pull out the two bytes (or fall through to error)
    if let [a, b] = tag.as_bytes() {
        // both must be ascii_alphanumeric and of the same case
        if !(a.is_ascii_alphanumeric()
            && b.is_ascii_alphanumeric()
            && (b.is_ascii_digit() || a.is_ascii_uppercase() == b.is_ascii_uppercase()))
        {
            errors.push(ParseMessage {
                line,
                code: ParseMessageCode::InvalidOptionalFieldTag,
                offender: tag.to_string(),
            });
        }
    } else {
        errors.push(ParseMessage {
            line,
            code: ParseMessageCode::InvalidOptionalFieldTag,
            offender: tag.to_string(),
        });
        return (None, errors);
    }

    // type should be a single character and match [AifZJHB]
    let type_char = type_str.chars().next().unwrap_or('Z');

    let ftype;
    if type_str.len() != 1 {
        errors.push(ParseMessage {
            line,
            code: ParseMessageCode::InvalidOptionalFieldType,
            offender: type_str.to_string(),
        });

        // fallback to string, don't try to use the first character as the type
        ftype = FieldType::String;
    } else {
        ftype = match FieldType::try_from(type_char) {
            Ok(t) => t,
            Err(code) => {
                errors.push(ParseMessage {
                    line,
                    code,
                    offender: type_str.to_string(),
                });

                // first char is unknown, fallback to string
                FieldType::String
            }
        };
    }

    // if the tag is reserved, check if it's being used for the right record type
    if tag.chars().next().unwrap().is_uppercase() {
        if let Err(e) = check_optional_field_tag_context(line, record_type, ftype, tag) {
            errors.push(e);
        }
    }

    // parse the value based on the type
    let (value_opt, mut val_errs) = parse_optional_field_value(line, ftype, value);
    errors.append(&mut val_errs);

    if let Some(value) = value_opt {
        let field = OptionalField {
            tag: tag.to_string(),
            type_: ftype,
            value,
        };
        (Some(field), errors)
    } else {
        (None, errors)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TagMap(pub HashMap<String, OptionalFieldValue>);

impl TagMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    #[inline]
    pub fn from_vec(tags: Vec<OptionalField>) -> Self {
        let mut map = HashMap::with_capacity(tags.len());
        for t in tags {
            map.insert(t.tag, t.value);
        }
        Self(map)
    }

    // creates a Vec<OptionalField> from the TagMap
    pub fn to_vec(&self) -> Vec<OptionalField> {
        self.0
            .iter()
            .map(|(k, v)| OptionalField {
                tag: k.clone(),
                type_: v.get_field_type(),
                value: v.clone(),
            })
            .collect()
    }

    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: for<'a> TryFrom<&'a OptionalFieldValue>,
    {
        self.0.get(key).and_then(|v| T::try_from(v).ok())
    }

    #[inline]
    pub fn contains(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    // uses the PF tag to store data as space delimited strings
    // TODO: could the tag name be an option?
    pub fn add_flag(&mut self, flag: &str) {
        if let Some(value) = self.0.get_mut("PF") {
            if let OptionalFieldValue::String(flags) = value {
                if !flags.split_whitespace().any(|f| f == flag) {
                    flags.push_str(&format!(" {flag}"));
                }
            }
        } else {
            self.0.insert("PF".to_string(), OptionalFieldValue::String(flag.to_string()));
        }
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        if let Some(value) = self.0.get("PF") {
            if let OptionalFieldValue::String(flags) = value {
                return flags.split_whitespace().any(|f| f == flag);
            }
        }
        false
    }

    pub fn add_tag(&mut self, tag: &str, value: OptionalFieldValue) {
        self.0.insert(tag.to_string(), value);
    }

    pub fn remove_tag(&mut self, tag: &str) {
        self.0.remove(tag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ParseMessageCode;
    use std::convert::TryFrom;

    #[test]
    fn test_fieldtype_try_from_valid() {
        // each type should convert correctly
        assert_eq!(FieldType::try_from('A'), Ok(FieldType::Char));
        assert_eq!(FieldType::try_from('i'), Ok(FieldType::Int));
        assert_eq!(FieldType::try_from('f'), Ok(FieldType::Float));
        assert_eq!(FieldType::try_from('Z'), Ok(FieldType::String));
        assert_eq!(FieldType::try_from('J'), Ok(FieldType::Json));
        assert_eq!(FieldType::try_from('H'), Ok(FieldType::ByteArray));
        assert_eq!(FieldType::try_from('B'), Ok(FieldType::NumberArray));
    }

    #[test]
    fn test_fieldtype_try_from_invalid() {
        // invalid type should return error
        let err = FieldType::try_from('X').unwrap_err();
        assert_eq!(err, ParseMessageCode::InvalidOptionalFieldType);
    }

    #[test]
    fn test_check_optional_field_tag_context() {
        // valid reserved
        assert!(check_optional_field_tag_context(1, &'H', FieldType::String, "VN").is_ok());

        // wrong type
        let e = check_optional_field_tag_context(1, &'H', FieldType::Int, "VN").unwrap_err();
        assert_eq!(e.code, ParseMessageCode::InvalidOptionalFieldReservedTagType);

        // wrong record
        let e2 = check_optional_field_tag_context(1, &'S', FieldType::String, "VN").unwrap_err();
        assert_eq!(e2.code, ParseMessageCode::UnexpectedReservedTagType);
    }

    // tests for parse_optional_field_value()

    #[test]
    fn test_parse_optional_field_value_empty() {
        let (opt, errs) = parse_optional_field_value(123, FieldType::String, "");
        
        assert!(opt.is_none()); // this is not a valid optional field, so we should get None
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, ParseMessageCode::OptionalFieldValueEmpty);
        assert_eq!(errs[0].line, 123);
    }

    #[test]
    fn test_parse_optional_field_value_char() {
        let (opt, errs) = parse_optional_field_value(1, FieldType::Char, "X");
        assert!(errs.is_empty()); // valid optional field, so no errors

        // the value should be Some(Char('X'))
        match opt.unwrap() {
            OptionalFieldValue::Char(c) => assert_eq!(c, 'X'),
            _ => panic!("expected Char"),
        }

        // this isn't valid, chars must be a single character
        let (opt2, errs2) = parse_optional_field_value(1, FieldType::Char, "XY");
        assert!(opt2.is_none()); // invalid OF, so None
        assert_eq!(errs2.len(), 1); 
        assert_eq!(
            errs2[0].code,
            ParseMessageCode::OptionalFieldValueTypeMismatch // check if correct error code
        );
    }

    #[test]
    fn test_parse_optional_field_value_int() {
        // valid integer
        let (opt, errs) = parse_optional_field_value(1, FieldType::Int, "123");
        assert!(errs.is_empty());
        
        match opt.unwrap() {
            OptionalFieldValue::Int(i) => assert_eq!(i, 123),
            _ => panic!("expected Int"),
        }

        // invalid integer
        let (opt2, errs2) = parse_optional_field_value(1, FieldType::Int, "abc");
        assert!(opt2.is_none());
        assert_eq!(
            errs2[0].code,
            ParseMessageCode::OptionalFieldValueTypeMismatch
        );
    }

    #[test]
    fn test_parse_optional_field_value_float() {
        // valid float
        let (opt, errs) = parse_optional_field_value(1, FieldType::Float, "1.23");
        assert!(errs.is_empty());
        
        match opt.unwrap() {
            OptionalFieldValue::Float(f) => assert!((f - 1.23).abs() < 1e-6),
            _ => panic!("expected Float"),
        }

        // invalid float
        let (opt2, errs2) = parse_optional_field_value(1, FieldType::Float, "abc");
        assert!(opt2.is_none());
        assert_eq!(
            errs2[0].code,
            ParseMessageCode::OptionalFieldValueTypeMismatch
        );
    }

    #[test]
    fn test_parse_optional_field_value_string_and_json() {
        // valid string
        let (opt_s, errs_s) = parse_optional_field_value(1, FieldType::String, "hello");
        assert!(errs_s.is_empty());
        match opt_s.unwrap() {
            OptionalFieldValue::String(s) => assert_eq!(s, "hello"),
            _ => panic!("expected String"),
        }

        // valid JSON
        let (opt_j, errs_j) = parse_optional_field_value(1, FieldType::Json, "{\"a\":1}");
        assert!(errs_j.is_empty());
        match opt_j.unwrap() {
            OptionalFieldValue::Json(s) => assert_eq!(s, "{\"a\":1}"),
            _ => panic!("expected Json"),
        }

        // TODO: if you ever do anything more with JSON, rewrite this test
    }

    #[test]
    fn test_parse_optional_field_value_bytearray() {
        let data = "hi".as_bytes().to_vec();
        let (opt, errs) = parse_optional_field_value(1, FieldType::ByteArray, "hi");
        assert!(errs.is_empty());
        
        match opt.unwrap() {
            OptionalFieldValue::ByteArray(v) => assert_eq!(v, data),
            _ => panic!("expected ByteArray"),
        }
    }

    #[test]
    fn test_parse_optional_field_value_number_array() {
        // invalid numberarray
        let (opt, errs) = parse_optional_field_value(1, FieldType::NumberArray, "1,2.5,foo");
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].offender, "foo"); // should report the invalid part
        
        // the valid parts should still be parsed
        if let OptionalFieldValue::NumberArray(arr) = opt.unwrap() {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0], OptionalFieldNumber::Int32(1));
            match arr[1] {
                OptionalFieldNumber::Float32(f) => assert!((f - 2.5).abs() < 1e-6),
                _ => panic!("expected Float32"),
            }
        } else {
            panic!("expected NumberArray");
        }
    }

    // tests for parse_optional_field()

    #[test]
    fn test_parse_optional_field_success_user_defined() {
        let (opt, errs) = parse_optional_field(1, &'X', "aa:i:42");
        assert!(errs.is_empty()); // user-defined optional fields should have no errors and parse successfully

        // check everything was parsed correctly
        let field = opt.unwrap();
        assert_eq!(field.tag, "aa");
        assert_eq!(field.type_, FieldType::Int);
        match field.value {
            OptionalFieldValue::Int(i) => assert_eq!(i, 42),
            _ => panic!("expected Int"),
        }
    }

    #[test]
    fn test_parse_optional_field_invalid_format() {
        let (opt, errs) = parse_optional_field(1, &'X', "badfield");
        assert!(opt.is_none());
        assert_eq!(errs[0].code, ParseMessageCode::InvalidOptionalField);
    }

    #[test]
    fn test_parse_optional_field_bad_tag() {
        let (opt, errs) = parse_optional_field(1, &'X', "A:i:1");
        assert!(opt.is_none());
        assert_eq!(errs[0].code, ParseMessageCode::InvalidOptionalFieldTag);
    }

    #[test]
    fn test_parse_optional_field_invalid_type_two_chars() {
        let (opt, errs) = parse_optional_field(1, &'X', "aa:ii:hello");
        // invalid type length
        assert!(errs.iter()
                .any(|e| e.code == ParseMessageCode::InvalidOptionalFieldType));
        // fallback to String, so succeeds with String("hello")
        assert!(opt.is_some());
        let field = opt.unwrap();
        assert_eq!(field.type_, FieldType::String);
        match field.value {
            OptionalFieldValue::String(s) => assert_eq!(s, "hello"),
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn test_parse_optional_field_type_mismatch() {
        // type is int, but value is non-integer, should return an error
        let (opt, errs) = parse_optional_field(1, &'X', "aa:i:xyz");
        assert!(opt.is_none());
        assert!(
            errs.iter()
                .any(|e| e.code == ParseMessageCode::OptionalFieldValueTypeMismatch)
        );
    }

    #[test]
    fn test_parse_optional_field_reserved_tag_ok() {
        // no errors expected, should parse correctly
        let (opt, errs) = parse_optional_field(1, &'H', "VN:Z:ver1");
        assert!(errs.is_empty());

        let field = opt.unwrap();
        assert_eq!(field.tag, "VN");
        assert_eq!(field.type_, FieldType::String);
        if let OptionalFieldValue::String(s) = field.value {
            assert_eq!(s, "ver1");
        } else {
            panic!("expected String");
        }
    }

    #[test]
    fn test_parse_optional_field_reserved_tag_unexpected_context() {
        // cannot use VN tag in a segment record; expect a warning but still parse it
        let (opt, errs) = parse_optional_field(1, &'S', "VN:Z:ver2");
        assert!(opt.is_some());

        assert!(
            errs.iter()
                .any(|e| e.code == ParseMessageCode::UnexpectedReservedTagType)
        );
    }
}
