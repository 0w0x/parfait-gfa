use crate::gfa::GFAVersion;
use crate::gfa::MissingBridgeOptions;
use crate::gfa::ParseOptions;
use crate::line::path::Step;

use crate::errors::ParseMessageCode;

use crate::errors::ParseMessage;

use crate::gfa::GfaParser;
use crate::line::utils::build_gfa_line;
use crate::optional_field::TagMap;

use crate::gfa::MissingSegmentOptions;

#[derive(Debug, Clone, Default)]
pub struct Walk {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub sample_id: String,
    pub hap_index: u32,
    pub seq_id: String,
    pub seq_start: Option<u32>,
    pub seq_end: Option<u32>,
    pub walk: Vec<Step>,
}

pub static REQ_COLUMNS_WALK: usize = 7;

impl Walk {
    pub fn parse_line(
        (gfa, parts, raw, n, map, options): (
            &mut GfaParser,
            &[&str],
            &str,
            usize,
            &mut TagMap,
            &ParseOptions,
        ),
    ) -> (Option<Self>, Vec<ParseMessage>) {
        let mut errors = vec![];

        let sample_id = parts.get(1).unwrap_or(&"").to_string();

        let hap_index = parts
            .get(2)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or_else(|| {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidHaplotypeIndex,
                    parts.get(2).unwrap_or(&"").to_string(),
                ));
                0
            });

        let seq_id: String = parts.get(3).unwrap_or(&"").to_string();

        let mut seq_start_is_asterisk = false;
        let seq_start = parts
            .get(4)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or_else(|| {
                if parts.get(4).expect("col count checked in record.rs") != &"*" {
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::InvalidSequenceStart,
                        parts.get(4).unwrap_or(&"").to_string(),
                    ));
                }
                seq_start_is_asterisk = true;
                0
            });

        let mut seq_end_is_asterisk = false;
        let seq_end = parts
            .get(5)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or_else(|| {
                if parts.get(5).expect("col count checked in record.rs") != &"*" {
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::InvalidSequenceEnd,
                        parts.get(5).unwrap_or(&"").to_string(),
                    ));
                }
                seq_end_is_asterisk = true;
                0
            });

        if seq_start > seq_end {
            errors.push(ParseMessage::new(
                n,
                ParseMessageCode::InvalidSequenceRange,
                format!("start ({seq_start}) > end ({seq_end})"),
            ));
        }

        // records with the same sample_id, hap_index, and seq_id are allowed
        // but their seq_start and seq_end must not overlap
        gfa.walks().for_each(|walk| {
            if walk.sample_id == sample_id
                && walk.hap_index == hap_index
                && walk.seq_id == seq_id
            {
                if let Some(existing_start) = walk.seq_start {
                    if let Some(existing_end) = walk.seq_end {
                        if (seq_start <= existing_end && seq_end >= existing_start) ||
                            (existing_start <= seq_end && existing_end >= seq_start) {
                            errors.push(ParseMessage::new(
                                n,
                                ParseMessageCode::OverlappingWalkRange,
                                format!(
                                    "{}/{}/{} with range {}..{} overlaps with {}..{} on line {}",
                                    sample_id, hap_index, seq_id,
                                    seq_start, seq_end,
                                    existing_start, existing_end,
                                    walk.line_no
                                ),
                            ));
                        }
                    }
                }
            }
        });

        let walk_str = parts.get(6).unwrap_or(&"");
        let mut walk_steps: Vec<Step> = vec![];
        let mut current_segment_name = vec![];
        let mut curr_step_direction = false;

        let walk_str_len = walk_str.len() as isize;
        let mut char_idx = -1;

        for c in walk_str.chars() {
            char_idx += 1;

            if char_idx == 0 {
                if !(c == '>' || c == '<') {
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::InvalidWalk,
                        walk_str.chars().take(100).collect(),
                    ));

                    return (None, errors);
                }

                curr_step_direction = c == '>';
                continue;
            }

            if c == '>' || c == '<' || char_idx == walk_str_len - 1 {
                // if this is the last character, add it to the current segment name
                // and continue to process the step
                if char_idx == walk_str_len - 1 {
                    current_segment_name.push(c);
                }

                // the start of a new step
                if !current_segment_name.is_empty() {
                    let segment_name = current_segment_name.iter().collect::<String>();
                    let segment = gfa.find_segment_with_name(&segment_name.clone());
                    let mut segment_id = 0; // this should always be mutated, i just dont want to use a match block 

                    if segment.is_some() {
                        segment_id = segment.unwrap().line_no as u32;
                    } else {
                        errors.push(ParseMessage::new(
                            n,
                            ParseMessageCode::SegmentNotFound,
                            segment_name.clone(),
                        ));

                        if options.handle_missing_segment == MissingSegmentOptions::HardSkip {
                            errors.push(ParseMessage::new(
                                n,
                                ParseMessageCode::InvalidWalk,
                                walk_str.chars().take(100).collect(),
                            ));
                            return (None, errors);
                        }

                        if options.handle_missing_segment == MissingSegmentOptions::CreateGhost {
                            let ghost = &gfa.create_ghost_segment(segment_name.clone());

                            segment_id = ghost.line_no as u32;
                        }
                    }

                    if segment_id != 0 || (segment_id == 0 && options.handle_missing_segment == MissingSegmentOptions::Ignore) {
                        walk_steps.push(Step {
                            segment_id,
                            orientation: curr_step_direction,
                        });
                    }

                    // compare the last two walk steps and see if a link exists between them         
                    if walk_steps.len() >= 2 {
                        let this_step = &walk_steps[walk_steps.len() - 1];
                        let last_step = &walk_steps[walk_steps.len() - 2];

                        let last_step_name = &gfa
                            .find_segment_mut(last_step.segment_id as usize)
                            .map(|s| s.name.clone())
                            .unwrap_or_default();

                        let is_valid = gfa.is_step_valid(
                            n,
                            last_step_name,
                            &segment_name.clone(),
                            last_step.orientation,
                            this_step.orientation,
                            true,
                            false,
                            false,
                            true,
                            true,
                        );

                        if !is_valid {
                            errors.push(ParseMessage::new(
                                n,
                                ParseMessageCode::LinkNotFound,
                                format!(
                                    "{}{} -> {}{}",
                                    last_step_name,
                                    if last_step.orientation { "+" } else { "-" },
                                    segment_name,
                                    if this_step.orientation { "+" } else { "-" }
                                ),
                            ));
                            
                            if options.handle_missing_bridge == MissingBridgeOptions::HardSkip {
                                errors.push(ParseMessage::new(
                                    n,
                                    ParseMessageCode::InvalidWalk,
                                    walk_str.chars().take(100).collect(),
                                ));
                                return (None, errors);
                            }

                            if options.handle_missing_bridge == MissingBridgeOptions::CreateGhostLink {
                                let _ = &gfa.create_ghost_link(
                                    last_step_name.clone(),
                                    last_step.orientation,
                                    segment_name.clone(),
                                    this_step.orientation,
                                    "*".to_string(),
                                );
                            }
                        }
                    }

                    current_segment_name.clear();
                    curr_step_direction = c == '>';
                } else {
                    // walk is starting a new step without a segment ID for the last step
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::InvalidWalk,
                        walk_str.chars().take(100).collect(),
                    ));
                    return (None, errors);
                }
            } else {
                current_segment_name.push(c);
            }
        }

        (
            Some(Self {
                line_no: n,
                raw: raw.to_owned(),
                tags: map.clone(),

                sample_id,
                hap_index,
                seq_id,
                seq_start: if seq_start_is_asterisk {
                    None
                } else {
                    Some(seq_start)
                },
                seq_end: if seq_end_is_asterisk {
                    None
                } else {
                    Some(seq_end)
                },
                walk: walk_steps,
            }),
            errors,
        )
    }

    pub fn to_raw_line(&self, _: GFAVersion, gfa: &GfaParser) -> String {
        self.to_raw_line_v1(gfa)
    }

    fn to_raw_line_v1(&self, gfa: &GfaParser) -> String {
        let sample_id = &self.sample_id;
        let hap_index = self.hap_index.to_string();
        let seq_id = &self.seq_id;

        let seq_start = match self.seq_start {
            Some(start) => start.to_string(),
            None => "*".to_string(),
        };

        let seq_end = match self.seq_end {
            Some(end) => end.to_string(),
            None => "*".to_string(),
        };

        let walk_str = self.walk
        .iter()
        .map(|step| {
            let step_id = step.segment_id as usize;
            format!(
                "{}{}",
                if step.orientation { '>' } else { '<' },
                gfa.find_segment(step_id).as_ref().map_or_else(
                    || step_id.to_string(),
                    |s| s.name.clone()
                )
            )
        }).collect::<Vec<String>>().join("");

        build_gfa_line(
            'W',
            &[
                sample_id,
                &hap_index,
                seq_id,
                &seq_start,
                &seq_end,
                &walk_str,
            ],
            &self.tags,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{errors::ParseMessageSeverity, gfa::{GfaParser, ParseOptions}};

    #[test]
    fn test_working_walks() {
        let mut gfa = GfaParser::new();

        let _ = gfa.parse("test/walk.gfa", &ParseOptions::default());
    
        gfa.messages.iter().for_each(|e| {
            e.print_formatted_error();
        });

        let has_errors = gfa.messages.iter().any(|e| e.severity() != ParseMessageSeverity::Warn && e.severity() != ParseMessageSeverity::Info);
        
        // TODO: write real test for walk
        assert!(!has_errors);
    }
}