use owo_colors::{AnsiColors, OwoColorize};
use std::fmt::Write;

#[derive(Debug, Clone, Default)]
pub struct ParseMessage {
    pub line: usize,
    pub code: ParseMessageCode,
    pub offender: String,
}

/// Severity levels for parse errors.
/// 
/// - Info: something to consider
/// - Warn: something that's not ideal
/// - Severe: something that could break other tools, but can still be parsed
/// - Error: something that cannot be parsed, skip this line
/// - Fatal: whole file is cooked
#[derive(PartialEq, Debug)]
pub enum ParseMessageSeverity {
    Info,
    Warn,
    Severe,
    Error,
    Fatal,
}

impl ParseMessageSeverity {
    pub fn as_str(&self) -> &str {
        match self {
            ParseMessageSeverity::Info => "*",
            ParseMessageSeverity::Warn => "?",
            ParseMessageSeverity::Severe => "#",
            ParseMessageSeverity::Error => "!",
            ParseMessageSeverity::Fatal => "X",
        }
    }
    pub fn to_char(&self) -> char {
        match self {
            ParseMessageSeverity::Info => 'i',
            ParseMessageSeverity::Warn => 'w',
            ParseMessageSeverity::Severe => 's',
            ParseMessageSeverity::Error => 'e',
            ParseMessageSeverity::Fatal => 'f',
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum ParseMessageCode {
    #[default]
    UnspecifiedError,
    InvalidOptionalField,
    InvalidOptionalFieldTag,
    InvalidOptionalFieldType,
    OptionalFieldValueTypeMismatch,
    InvalidOptionalFieldReservedTagType,
    DuplicateOptionalField,
    OptionalFieldValueEmpty,
    UnexpectedReservedTagType,
    InvalidLine,
    IOError,
    DirectoryError,
    UnknownLine,
    MissingVersionTag,
    UnknownVersion,
    DuplicateHeader,
    MissingHeader,
    HeaderNotOnFirstLine,
    SegmentLengthMismatch,
    InvalidSequenceLength,
    NamespaceCollision,
    RedundantSegmentLengthTag,
    RedundantSegmentLengthTagMismatch,
    InvalidSequence,
    IndeterminateSegmentLength,
    SegmentNotFound,
    InvalidOrientation,
    InvalidCIGAR,
    InvalidJumpDistance,
    InvalidShortcut,
    InvalidID,
    InvalidPosition,
    InvalidContainmentPositionRange,
    InvalidExternalReference,
    SelfContainment,
    IsolatedSegment,
    DeadEndTip,
    SelfBridge,
    PathOverlapLengthMismatch,
    InvalidPath,
    InvalidPathStep,
    InvalidPathStepOrientation,
    LinkNotFound,
    BridgeGoesNowhere,
    InvalidHaplotypeIndex,
    InvalidSequenceStart,
    InvalidSequenceEnd,
    InvalidSequenceRange,
    OverlappingWalkRange,
    InvalidWalkStep,
    InvalidWalk,
    WalkLinkHasOverlap,
    InvalidDirectedReference,
    InvalidIntervalPosition,
    InvalidIntervalPositionRange,
    InvalidIntervalPositionSentinel,
    MissingIntervalPositionSentinel,
    InvalidAlignment,
    RedundantEdgeIDTag,
    EdgeIDTagUsedInAnonEdge,
    InvalidGapDistance,
    InvalidVariance,
    GroupMemberNotFound,
    InvalidGroup,
}

impl std::fmt::Display for ParseMessageCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::fmt::Display for ParseMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = self.formatted();
        write!(f, "{formatted_error}")
    }
}

impl ParseMessageSeverity {
    fn colours(&self) -> (owo_colors::AnsiColors, owo_colors::AnsiColors) {
        use ParseMessageSeverity::*;
        match self {
            Info => (AnsiColors::Blue, AnsiColors::BrightBlue),
            Warn => (AnsiColors::Yellow, AnsiColors::BrightYellow),
            Severe => (AnsiColors::Red, AnsiColors::BrightRed),
            Error => (AnsiColors::BrightRed, AnsiColors::Red),
            Fatal => (AnsiColors::Magenta, AnsiColors::BrightMagenta),
        }
    }

    fn header(&self) -> String {
        let (fg, bg) = self.colours();
        format!("[{}]", self.as_str())
            .color(fg)
            .on_color(bg)
            .to_string()
    }

    fn body<T: AsRef<str>>(&self, text: T) -> String {
        let (fg, _) = self.colours();
        text.as_ref().color(fg).to_string()
    }
}

impl ParseMessage {
    pub fn new(line: usize, code: ParseMessageCode, offender: String) -> Self {
        Self {
            line,
            code,
            offender,
        }
    }

    fn formatted(&self) -> String {
        let (severity, message) = self.get_message();

        let header = severity.header();
        let code = severity.body(format!("[parfait-gfa] {:?}", self.code));
        let context = severity.body(format!(
            "while parsing {}{} on line {}",
            self.offender.chars().take(256).collect::<String>(),
            if self.offender.len() > 256 { "..." } else { "" },
            self.line
        ));
        let msg = severity.body(message);

        let mut out = String::new();

        writeln!(&mut out, "{} {}", header.bold(), code.bold()).unwrap();
        writeln!(&mut out, "{msg}").unwrap();
        writeln!(&mut out, "{}", context.italic()).unwrap();
        writeln!(&mut out).unwrap();
        out
    }

    pub fn print_formatted_error(&self) {
        let formatted_error: String = self.formatted();
        print!("{formatted_error}");
    }

    // TODO: rework entire error system
    // right now errors are missing...
    // - custom info at the time
    // - raw is unused
    // - line number doesn't bubble up

    fn get_message(&self) -> (ParseMessageSeverity, String) {
        match self.code {
            ParseMessageCode::InvalidLine => (
                ParseMessageSeverity::Error,
                "failed to parse line".to_string(),
            ),
            ParseMessageCode::InvalidOptionalField => (
                ParseMessageSeverity::Severe,
                "optional field is not a valid SAM tag".to_string(),
            ),
            ParseMessageCode::InvalidOptionalFieldType => (
                ParseMessageSeverity::Warn,
                "optional field type must match /[AifZJHB]/; defaulting value to string".to_string(),
            ),
            ParseMessageCode::InvalidOptionalFieldTag => (
                ParseMessageSeverity::Warn,
                "optional field tags must match /[A-Za-z][A-Za-z0-9]/".to_string(),
            ),
            ParseMessageCode::InvalidOptionalFieldReservedTagType => (
                ParseMessageSeverity::Warn,
                "reserved optional field tags must use their correct type defined in the spec".to_string(),
            ),
            ParseMessageCode::UnexpectedReservedTagType => (
                ParseMessageSeverity::Warn,
                "this tag type is not expected in this context".to_string(),
            ),
            ParseMessageCode::OptionalFieldValueTypeMismatch => (
                ParseMessageSeverity::Severe,
                "optional field value cannot be parsed with the given type".to_string(),
            ),
            ParseMessageCode::OptionalFieldValueEmpty => (
                ParseMessageSeverity::Warn,
                "optional field value is empty".to_string(),
            ),
            ParseMessageCode::DuplicateOptionalField => (
                ParseMessageSeverity::Severe,
                "duplicate optional field tag found in the same record; defaulting to last occurrence".to_string(),
            ),
            ParseMessageCode::IOError => (
                ParseMessageSeverity::Fatal,
                "an I/O error occurred while reading the file".to_string(),
            ),
            ParseMessageCode::DirectoryError => (
                ParseMessageSeverity::Fatal,
                "provided path is a directory".to_string(),
            ),
            ParseMessageCode::UnspecifiedError => (
                ParseMessageSeverity::Fatal,
                "an unspecified error occurred".to_string(),
            ),
            ParseMessageCode::UnknownLine => (
                ParseMessageSeverity::Info,
                "unknown line type encountered".to_string(),
            ),
            ParseMessageCode::MissingVersionTag => (
                ParseMessageSeverity::Warn,
                "missing version tag in the header line; defaulting to v1".to_string(), // TODO: automatically determine version based on records
            ),
            ParseMessageCode::UnknownVersion => (
                ParseMessageSeverity::Severe,
                "unknown/unsupported GFA version (expected 1, 1.0, 1.1, 1.2, 2.0, or 2); defaulting to 1.0".to_string(),
            ),
            ParseMessageCode::DuplicateHeader => (
                ParseMessageSeverity::Warn,
                "duplicate header line found; this is allowed but only the first one will be used".to_string(),
            ),
            ParseMessageCode::MissingHeader => (
                ParseMessageSeverity::Warn,
                "missing header line; GFA files should ideally start with a header".to_string(),
            ),
            ParseMessageCode::HeaderNotOnFirstLine => (
                ParseMessageSeverity::Warn,
                "header should ideally be on the first line".to_string(),
            ),
            ParseMessageCode::SegmentLengthMismatch => (
                ParseMessageSeverity::Severe,
                "segment length tag (LN) does not match the length of the sequence".to_string(),
            ),
            ParseMessageCode::InvalidSequenceLength => (
                ParseMessageSeverity::Severe,
                "v2 sequence length could not be parsed; defaulting to the length of the sequence column".to_string(),
            ),
            ParseMessageCode::InvalidSequenceRange => (
                ParseMessageSeverity::Severe,
                "seq_start must be less than or equal to seq_end".to_string(),
            ),
            ParseMessageCode::NamespaceCollision => (
                ParseMessageSeverity::Severe,
                "duplicate record reference found in the same namespace; appending occurrence to reference".to_string(),
            ),
            ParseMessageCode::RedundantSegmentLengthTag => (
                ParseMessageSeverity::Warn,
                "redundant length tag (LN) in v2 segment; the length column was successfully parsed, so this tag is not needed".to_string(),
            ),
            ParseMessageCode::RedundantSegmentLengthTagMismatch => (
                ParseMessageSeverity::Warn,
                "redundant length tag (LN) in v2 segment and LN does not match length column; defaulting to v2 length column".to_string(),
            ),
            ParseMessageCode::InvalidSequence => (
                ParseMessageSeverity::Severe,
                "sequence must match * or [!-~]+".to_string(),
            ),
            ParseMessageCode::IndeterminateSegmentLength => (
                ParseMessageSeverity::Severe,
                "sequence was not provided (*) and no length tag (LN) was found; defaulting length to 1".to_string(),
            ),
            ParseMessageCode::SegmentNotFound => (
                ParseMessageSeverity::Severe,
                "referenced segment does not exist within the graph".to_string(),
            ),
            ParseMessageCode::InvalidOrientation => (
                ParseMessageSeverity::Severe,
                "orientation must match [+-]; defaulting to +".to_string(),
            ),
            ParseMessageCode::InvalidCIGAR => (
                ParseMessageSeverity::Severe,
                "overlap CIGAR string must match /[0-9]+[MIDNSHPX=]/; defaulting to *".to_string(),
            ),
            ParseMessageCode::InvalidJumpDistance => (
                ParseMessageSeverity::Severe,
                "jump distance must be a signed integer or omitted; defaulting to *".to_string(),
            ),
            ParseMessageCode::InvalidGapDistance => (
                ParseMessageSeverity::Severe,
                "gap distance must be a signed integer; defaulting to 0".to_string(),
            ),
            ParseMessageCode::InvalidShortcut => (
                ParseMessageSeverity::Severe,
                "jump shortcut must be either 0 or 1; defaulting to 0".to_string(),
            ),
            ParseMessageCode::InvalidPosition => (
                ParseMessageSeverity::Severe,
                "position must be a non-negative integer; defaulting to 0".to_string(),
            ),
            ParseMessageCode::InvalidContainmentPositionRange => (
                ParseMessageSeverity::Severe,
                "position is outside the bounds of the segment length".to_string(),
            ),
            ParseMessageCode::SelfContainment => (
                ParseMessageSeverity::Warn,
                "encountered a self-containment; a segment cannot contain itself".to_string(),
            ),
            ParseMessageCode::IsolatedSegment => (
                ParseMessageSeverity::Info,
                "segment is not referenced by any bridge records".to_string(),
            ),
            ParseMessageCode::DeadEndTip => (
                ParseMessageSeverity::Info,
                "segment is a dead-end tip (missing an outgoing or incoming bridge, or both)".to_string(),
            ),
            ParseMessageCode::SelfBridge => (
                ParseMessageSeverity::Warn,
                "bridge connects the segment to itself".to_string(),
            ),
            ParseMessageCode::PathOverlapLengthMismatch => (
                ParseMessageSeverity::Severe,
                "length of overlaps must be one less than segments; all overlaps will be ignored".to_string(),
            ),
            ParseMessageCode::InvalidPathStep => (
                ParseMessageSeverity::Severe,
                "path step must be a valid segment ID and orientation (e.g. 1+, 2-, 3+); skipping step".to_string(),
            ),
            ParseMessageCode::InvalidPathStepOrientation => (
                ParseMessageSeverity::Severe,
                "path step orientation must be either + or -; defaulting to +".to_string(),
            ),
            ParseMessageCode::LinkNotFound => (
                ParseMessageSeverity::Severe,
                "no link found between segments".to_string(),
            ),
            ParseMessageCode::BridgeGoesNowhere => (
                ParseMessageSeverity::Error,
                "both segments in the bridge do not exist; skipping bridge".to_string(),
            ),
            ParseMessageCode::InvalidHaplotypeIndex => (
                ParseMessageSeverity::Severe,
                "haplotype index must be a non-negative integer; defaulting to 0".to_string(),
            ),
            ParseMessageCode::InvalidSequenceStart => (
                ParseMessageSeverity::Severe,
                "sequence start must be a non-negative integer or *; defaulting to *".to_string(),
            ),
            ParseMessageCode::InvalidSequenceEnd => (
                ParseMessageSeverity::Severe,
                "sequence end must be a non-negative integer or *; defaulting to *".to_string(),
            ),
            ParseMessageCode::OverlappingWalkRange => (
                ParseMessageSeverity::Warn,
                "walks with the same sample_id, hap_index, and seq_id must have non-overlapping sequence ranges".to_string(),
            ),
            ParseMessageCode::InvalidWalkStep => (
                ParseMessageSeverity::Severe,
                "walk step must be a direction (<|>) followed by a valid segment ID; skipping step".to_string(),
            ),
            ParseMessageCode::InvalidWalk => (
                ParseMessageSeverity::Error,
                "could not parse walk steps; skipping walk line".to_string(),
            ),
            ParseMessageCode::InvalidPath => (
                ParseMessageSeverity::Error,
                "could not parse path steps; skipping path line".to_string(),
            ),
            ParseMessageCode::WalkLinkHasOverlap => (
                ParseMessageSeverity::Warn,
                "link connecting two walk steps has an overlap; walks are not intended for graphs with overlaps".to_string(),
            ),
            ParseMessageCode::InvalidID => (
                ParseMessageSeverity::Severe,
                "IDs must use printable ASCII characters and not be empty; defaulting to line number".to_string(),
            ),
            ParseMessageCode::InvalidDirectedReference => (
                ParseMessageSeverity::Error,
                "directed references must be valid (e.g. 1+ or 2-); skipping entire record".to_string(),
            ),
            ParseMessageCode::InvalidExternalReference => (
                ParseMessageSeverity::Severe,
                "external reference must be a valid directed reference (e.g. 1+ or 2-)".to_string(),
            ),
            ParseMessageCode::InvalidIntervalPosition => (
                ParseMessageSeverity::Error,
                "v2 interval positions must be signed integers with an optional postfix $; skipping entire record".to_string(),
            ),
            ParseMessageCode::InvalidIntervalPositionRange => (
                ParseMessageSeverity::Severe,
                "v2 interval position is out of bounds for the referenced segment".to_string(),
            ),
            ParseMessageCode::InvalidIntervalPositionSentinel => (
                ParseMessageSeverity::Severe,
                "when the postfix $ is used, the position must equal the length of the segment".to_string(),
            ),
            ParseMessageCode::MissingIntervalPositionSentinel => (
                ParseMessageSeverity::Severe,
                "interval position is the last base in the segment, a postfix $ is required".to_string(),
            ),
            ParseMessageCode::InvalidAlignment => (
                ParseMessageSeverity::Severe,
                "alignment must be either CIGAR or Trace; defaulting to *".to_string(),
            ),
            ParseMessageCode::RedundantEdgeIDTag => (
                ParseMessageSeverity::Warn,
                "redundant edge_id tag (ID) in v2 edge/gap. the id column was not omitted, so this tag is not needed; ignoring tag".to_string(),
            ),
            ParseMessageCode::EdgeIDTagUsedInAnonEdge => (
                ParseMessageSeverity::Warn,
                "id column was omitted but an edge_id tag was provided; setting id column to the tag value".to_string(),
            ),
            ParseMessageCode::InvalidVariance => (
                ParseMessageSeverity::Severe,
                "variance must be a signed integer or omitted; defaulting to *".to_string(),
            ),
            ParseMessageCode::GroupMemberNotFound => (
                ParseMessageSeverity::Severe,
                "group member not found in namespace".to_string(),
            ),
            ParseMessageCode::InvalidGroup => (
                ParseMessageSeverity::Severe,
                "could not parse group; skipping group line".to_string(),
            ),
        }
    }

    pub fn severity(&self) -> ParseMessageSeverity {
        let (s, _) = self.get_message();
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_symbols_are_correct() {
        assert_eq!(ParseMessageSeverity::Info.as_str(), "*");
        assert_eq!(ParseMessageSeverity::Warn.as_str(), "?");
        assert_eq!(ParseMessageSeverity::Severe.as_str(), "#");
        assert_eq!(ParseMessageSeverity::Error.as_str(), "!");
        assert_eq!(ParseMessageSeverity::Fatal.as_str(), "X");
    }

    #[test]
    fn header_contains_symbol_and_ansi_codes() {
        let header = ParseMessageSeverity::Error.header();
        assert!(header.contains("!"));
        assert!(header.contains("\u{1b}["));
    }

    #[test]
    fn formatted_error_contains_expected_bits() {
        let err = ParseMessage {
            line: 5,
            code: ParseMessageCode::UnexpectedReservedTagType,
            offender: "foo".into(),
        };

        let out = err.formatted();
        assert!(out.contains("[parfait-gfa]"));
        assert!(out.contains("this tag type is not expected in this context"));
        assert!(out.contains("?"));
    }
}
