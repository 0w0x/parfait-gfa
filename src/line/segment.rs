use crate::errors::ParseMessage;
use crate::errors::ParseMessageCode;
use crate::gfa::GFAVersion;
use crate::gfa::GfaParser;
use crate::line::utils::build_gfa_line;
use crate::line::utils::is_valid_name;
use crate::optional_field::OptionalFieldValue;
use crate::optional_field::TagMap;

#[derive(Debug, Clone)]
pub struct Segment {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub name: String,
    pub sequence: String,

    pub length: Option<i32>,

    pub outgoing_links: Vec<usize>,
    pub incoming_links: Vec<usize>,
    pub containments: Vec<usize>,
    pub contained_by: Vec<usize>,
    pub outgoing_jumps: Vec<usize>,
    pub incoming_jumps: Vec<usize>,
    pub outgoing_edges: Vec<usize>,
    pub incoming_edges: Vec<usize>,
    pub outgoing_gaps: Vec<usize>,
    pub incoming_gaps: Vec<usize>,
    pub fragments: Vec<usize>,
}

impl Default for Segment {
    fn default() -> Self {
        Self {
            line_no: 0,
            raw: "".to_string(),
            tags: TagMap::new(),

            name: "Segment".to_string(),
            sequence: "*".to_string(),
            length: None,

            outgoing_links: vec![],
            incoming_links: vec![],
            containments: vec![],
            contained_by: vec![],
            outgoing_jumps: vec![],
            incoming_jumps: vec![],
            outgoing_edges: vec![],
            incoming_edges: vec![],
            outgoing_gaps: vec![],
            incoming_gaps: vec![],
            fragments: vec![],
        }
    }
}
    
impl Segment {
    pub fn parse_line(
        (gfa, parts, raw, n, map, options): (
            &mut GfaParser,
            &[&str],
            &str,
            usize,
            &mut TagMap,
            &crate::gfa::ParseOptions,
        ),
    ) -> (Option<Self>, Vec<ParseMessage>) {
        let mut errors = vec![];

        if !is_valid_name(parts[1]) {
            errors.push(ParseMessage::new(
                n,
                ParseMessageCode::InvalidID,
                parts[1].to_owned(),
            ));
            
            return (None, errors);
        }

        let name = &gfa.ensure_name_unique(n, parts[1].to_owned());
        let ln_tag = map.get::<i32>("LN");

        let mut length = None;
        let sequence;

        let version: GFAVersion = gfa.version.clone();
        
        if version == GFAVersion::V2 {
            sequence = parts[3].to_owned(); // col 3 in v2

            // attempt to parse the length from the second column
            length = Some(parts[2].parse::<i32>().unwrap_or_else(|_| {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidSequenceLength,
                    raw.to_owned(),
                ));

                // fallback to the length of the sequence
                sequence.len() as i32
            }));

            // you don't need an LN tag in a gfa v2 file
            if ln_tag.is_some() {
                if Some(ln_tag) != Some(length) {
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::RedundantSegmentLengthTagMismatch,
                        raw.to_owned(),
                    ));
                } else {
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::RedundantSegmentLengthTag,
                        raw.to_owned(),
                    ));
                }
            }
        } else { // GFAVersion::V1
            sequence = parts[2].to_owned(); // col 2 in v1

            if ln_tag.is_some() {
                // v1 LN tag should probably match sequence length (when not *)
                if sequence != "*" && ln_tag != Some(sequence.len() as i32) {
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::SegmentLengthMismatch,
                        raw.to_owned(),
                    ));
                }
            } else if sequence == "*" || sequence.is_empty() {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::IndeterminateSegmentLength,
                    raw.to_owned(),
                ));
            }            
        }

        // check if sequence is valid, this can take a while for large sequences
        // there's probably a faster way to do this
        if !options.skip_invalid_sequence_test {
            let bytes = sequence.as_bytes();
            
            // the sequence must match * or [!-~]+
            if !(bytes.len() == 1 && bytes[0] == b'*')
                && bytes.iter().any(|&b| b < b'!' || b > b'~')
            {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidSequence,
                    raw.to_owned(),
                ));
            }
        }

        if !options.store_sequences {
            // if we're not storing sequences and there's no LN tag,
            // then create one from the sequence length
            if ln_tag.is_none() && version != GFAVersion::V2 && sequence != "*" {
                map.0.insert(
                    "LN".to_string(),
                    OptionalFieldValue::Int(sequence.len() as i32),
                );
            }
        }

        (
            Some(Self {
                line_no: n,
                raw: raw.to_owned(),
                tags: map.clone(),

                name: name.to_string(),
                sequence: if options.store_sequences {
                    sequence.to_owned()
                } else {
                    "*".to_string()
                },
                length,

                outgoing_links: vec![],
                incoming_links: vec![],
                containments: vec![],
                contained_by: vec![],
                outgoing_jumps: vec![],
                incoming_jumps: vec![],
                outgoing_edges: vec![],
                incoming_edges: vec![],
                outgoing_gaps: vec![],
                incoming_gaps: vec![],
                fragments: vec![],
            }),
            errors,
        )
    }

    pub fn get_length(&self) -> i32 {
        // priority for length:
        // 1. v2 length column
        // 2. LN tag
        // 3. sequence length (if not *)
        // otherwise, 0

        if self.length.is_some() {
            self.length.unwrap()
        } else if self.tags.get::<i32>("LN").is_some() {
            self.tags.get::<i32>("LN").unwrap()
        } else if self.sequence != "*" && !self.sequence.is_empty() {
            // if this overflows it will be the funniest thing ever
            self.sequence.len() as i32
        } else {
            0
        }
    }

    pub fn get_outgoing_bridges(&self) -> Vec<usize> {
        let mut bridges = vec![];
        bridges.extend(self.outgoing_links.iter());
        bridges.extend(self.outgoing_jumps.iter());
        bridges.extend(self.outgoing_edges.iter());
        bridges.extend(self.outgoing_gaps.iter());
        
        // i say we include containments since v2 edges
        // basically generalise them anyway
        bridges.extend(self.containments.iter());
        bridges
    }

    pub fn get_incoming_bridges(&self) -> Vec<usize> {
        let mut bridges = vec![];
        bridges.extend(self.incoming_links.iter());
        bridges.extend(self.incoming_jumps.iter());
        bridges.extend(self.incoming_edges.iter());
        bridges.extend(self.incoming_gaps.iter());

        // including contained_by because v2 edges
        bridges.extend(self.contained_by.iter());
        bridges
    }
    
    pub fn to_raw_line(&self, version: GFAVersion) -> String {
        match version {
            GFAVersion::V2 => self.to_raw_line_v2(),
            _ => self.to_raw_line_v1(),
        }
    }

    fn to_raw_line_v1(&self) -> String {
        let name = self.name.as_str();
        let sequence = self.sequence.as_str();
        
        build_gfa_line(
            'S',
            &[name, sequence],
            &self.tags,
        )
    }

    fn to_raw_line_v2(&self) -> String {
        let name = self.name.as_str();
        let sequence = self.sequence.as_str();

        // use get_length over self.length for v1 -> v2 conversions
        let length = self.get_length().to_string();

        build_gfa_line(
            'S',
            &[name, &length, sequence],
            &self.tags,
        )
    }
}

