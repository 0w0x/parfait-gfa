use crate::{
    errors::{ParseMessage, ParseMessageCode},
    line::segment::Segment,
    optional_field::TagMap,
};

#[derive(Debug, Clone, Default)]
pub struct IntervalPosition {
    pub position: i32,
    pub is_last: bool, // true if the position ends with a '$'
}

impl std::fmt::Display for IntervalPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_last {
            write!(f, "{}$", self.position)
        } else {
            write!(f, "{}", self.position)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DirectedReference {
    pub reference: String,
    pub direction: bool, // true for +, false for -
}

impl std::fmt::Display for DirectedReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.direction {
            write!(f, "{}+", self.reference)
        } else {
            write!(f, "{}-", self.reference)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Interval {
    pub begin: IntervalPosition,
    pub end: IntervalPosition,
}

impl std::fmt::Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.begin, self.end)
    }
}

pub fn is_valid_name(name: &str) -> bool {
    // A valid name must:
    // - use printable ASCII characters
    // - not be empty
    // - not contain spaces
    // - not contain +, or -,
    // - not start with * or =
    name.chars().all(|c| c.is_ascii_graphic())
        && !name.is_empty()
        && !name.contains(" ")
        && !name.contains("+,")
        && !name.contains("-,")
        && !name.starts_with('*')
        && !name.starts_with('=')
}

#[inline]
pub fn deduce_alignment(alignment: &str) -> Result<Option<Alignment>, ParseMessage> {
    if alignment == "*" {
        Ok(None)
    } else if is_valid_cigar(alignment) {
        Ok(Some(Alignment::CIGAR(alignment.to_owned())))
    } else if is_valid_trace(alignment) {
        Ok(Some(Alignment::Trace(alignment.to_owned())))
    } else {
        Err(ParseMessage::new(
            0,
            ParseMessageCode::InvalidAlignment,
            alignment.to_owned(),
        ))
    }
}

impl std::fmt::Display for Alignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Alignment::CIGAR(cigar) => write!(f, "{cigar}"),
            Alignment::Trace(trace) => write!(f, "{trace}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Alignment {
    CIGAR(String),
    Trace(String),
}

pub fn parse_directed_reference(reference: &str) -> Result<DirectedReference, ParseMessage> {
    // check if last char is + or -
    let last_char = reference.chars().last().ok_or_else(|| {
        ParseMessage::new(
            0,
            ParseMessageCode::InvalidDirectedReference,
            reference.to_owned(),
        )
    })?;

    let direction = match last_char {
        '+' => true,
        '-' => false,
        _ => {
            return Err(ParseMessage::new(
                0,
                ParseMessageCode::InvalidDirectedReference,
                reference.to_owned(),
            ));
        }
    };

    let name = &reference[..reference.len() - 1];
    if !is_valid_name(name) {
        return Err(ParseMessage::new(
            0,
            ParseMessageCode::InvalidDirectedReference,
            reference.to_owned(),
        ));
    }

    Ok(DirectedReference {
        reference: name.to_owned(),
        direction,
    })
}

pub fn parse_interval(
    n: usize,
    errors: &mut Vec<ParseMessage>,
    segment: Option<&Segment>,
    begin: &str,
    end: &str,
) -> Result<Interval, ()> {
    let errors_len: usize = errors.len();

    let begin = parse_position(n, errors, begin);
    let end = parse_position(n, errors, end);

    if errors_len != errors.len() {
        return Err(());
    }

    let interval = Interval {
        begin: begin.unwrap(),
        end: end.unwrap(),
    };

    if let Some(s) = segment {
        check_interval(n, errors, &interval, s);
    }

    Ok(interval)
}

fn check_interval(
    n: usize,
    errors: &mut Vec<ParseMessage>,
    interval: &Interval,
    segment: &Segment,
) {
    let name = segment.name.clone();
    let length = segment.get_length();

    // check if interval can be mapped to the segment
    if interval.begin.position > length || interval.end.position > length {
        errors.push(ParseMessage::new(
            n,
            ParseMessageCode::InvalidIntervalPositionRange,
            format!(
                "{} where L = {} for segment {}",
                interval,
                length,
                name.clone()
            ),
        ));
    }

    // check if postfix sentinel is used correctly
    if interval.begin.is_last && (interval.begin.position != length) {
        errors.push(ParseMessage::new(
            n,
            ParseMessageCode::InvalidIntervalPositionSentinel,
            format!(
                "segment begin: {} where L = {} for segment {}",
                interval.begin,
                length,
                name.clone()
            ),
        ));
    }

    if interval.end.is_last && (interval.end.position != length) {
        errors.push(ParseMessage::new(
            n,
            ParseMessageCode::InvalidIntervalPositionSentinel,
            format!(
                "segment end: {} where L = {} for segment {}",
                interval.end,
                length,
                name.clone()
            ),
        ));
    }

    // check if the sentinels are missing
    if !interval.begin.is_last && (interval.begin.position == length) {
        errors.push(ParseMessage::new(
            n,
            ParseMessageCode::MissingIntervalPositionSentinel,
            format!(
                "segment begin: {} where L = {} for segment {}",
                interval.begin,
                length,
                name.clone()
            ),
        ));
    }

    if !interval.end.is_last && (interval.end.position == length) {
        errors.push(ParseMessage::new(
            n,
            ParseMessageCode::MissingIntervalPositionSentinel,
            format!(
                "segment end: {} where L = {} for segment {}",
                interval.end,
                length,
                name.clone()
            ),
        ));
    }
}

#[inline]
pub fn parse_position(
    n: usize,
    errors: &mut Vec<ParseMessage>,
    position: &str,
) -> Result<IntervalPosition, ()> {
    // A valid position is a signed integer that can optionally end with a "$" character
    if position.is_empty() {
        errors.push(ParseMessage::new(
            n,
            ParseMessageCode::InvalidIntervalPosition,
            position.to_owned(),
        ));
        return Err(());
    }

    let has_postfix = position.ends_with('$');
    let pos_str = if has_postfix {
        &position[..position.len() - 1]
    } else {
        position
    };

    let pos: i32 = pos_str.parse().map_err(|_| {
        errors.push(ParseMessage::new(
            n,
            ParseMessageCode::InvalidIntervalPosition,
            position.to_owned(),
        ));
    })?;

    Ok(IntervalPosition {
        position: pos,
        is_last: has_postfix,
    })
}

#[inline]
pub fn is_valid_trace(trace: &str) -> bool {
    // A valid trace is defined as <int>(,<int>)* where each int is a signed integer
    if trace.is_empty() {
        return false;
    }

    let parts: Vec<&str> = trace.split(',').collect();
    for part in parts {
        if part.is_empty() {
            return false;
        }
        if part.parse::<i32>().is_err() {
            return false;
        }
    }

    true
}

#[inline]
pub fn is_valid_cigar(cigar: &str) -> bool {
    let bytes = cigar.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // scan digits
        if !bytes[i].is_ascii_digit() {
            return false; // no number found
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        // parse the operator
        match bytes.get(i) {
            Some(b'M' | b'I' | b'D' | b'N' | b'S' | b'H' | b'P' | b'X' | b'=') => i += 1,
            _ => return false, // any other character is invalid
        }
    }
    true
}

// TODO: profile these inlines
#[inline]
pub fn build_gfa_line(record_type: char, columns: &[&str], tags: &TagMap) -> String {
    let mut line = String::new();
    line.push(record_type);
    for col in columns {
        line.push('\t');
        line.push_str(col);
    }
    tags.0.iter().for_each(|(tag, value)| {
        line.push('\t');
        line.push_str(tag);
        line.push(':');
        line.push(value.get_field_type().get_char());
        line.push(':');
        line.push_str(value.to_string().as_str());
    });
    line
}
