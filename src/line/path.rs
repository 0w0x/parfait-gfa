use crate::gfa::GFAVersion;
use crate::gfa::MissingSegmentOptions;
use crate::gfa::ParseOptions;
use crate::line::utils::build_gfa_line;
use crate::line::utils::is_valid_cigar;

use crate::errors::ParseMessageCode;

use crate::errors::ParseMessage;

use crate::gfa::GfaParser;
use crate::line::utils::is_valid_name;
use crate::optional_field::TagMap;

#[derive(Debug, Clone, Default)]
pub struct Path {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub name: String,
    pub steps: Vec<Step>,
    pub overlaps: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub segment_id: u32,
    pub orientation: bool, // true for +, false for -
}

pub static REQ_COLUMNS_PATH: usize = 4;

impl Path {
    pub fn parse_line(
        (gfa, parts, raw, n, map, options): (
            &mut GfaParser,
            &[&str],
            &str,
            usize,
            &mut TagMap,
            &ParseOptions,
        )
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

        let name = &gfa.ensure_name_unique(n, parts[1].to_string());

        let steps_str = parts[2].split([',', ';']).collect::<Vec<&str>>();
        let mut overlaps_str = parts[3].split(",").collect::<Vec<&str>>();

        let mut steps: Vec<Step> = Vec::with_capacity(steps_str.len());
        let mut overlaps: Vec<String> = Vec::with_capacity(steps_str.len());

        let use_overlaps = if parts[3] == "*" {    
            // no overlaps provided
            false
        } else if steps_str.len() != (overlaps_str.len() + 1) {
            // overlaps must be one less than segments
            // don't use overlaps if there is a mismatch
            errors.push(ParseMessage::new(
                n,
                ParseMessageCode::PathOverlapLengthMismatch,
                raw.to_owned(),
            ));
            false
        } else {
            true
        };

        let mut prev_step = None::<Step>;
        let mut step_idx: isize = -1;

        // parse each step in the path
        for path_step in &steps_str {
            step_idx += 1;

            // shortest path step is 2 characters (A+)
            if path_step.len() < 2 {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidPathStep,
                    path_step.to_string(),
                ));
                continue;
            }

            let (segment, orientation) = if let Some(s) = path_step.strip_suffix('+') {
                (s.to_string(), true)
            } else if let Some(s) = path_step.strip_suffix('-') {
                (s.to_string(), false)
            } else {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidPathStepOrientation,
                    path_step.to_string(),
                ));
                (path_step.to_string(), true)
            };

            // we store the real overlap and set it to @
            // if everything checks out, we will replace it later
            let real_overlap = if use_overlaps {
                overlaps_str.get(step_idx as usize).unwrap_or(&"@")
            } else {
                "@"
            };

            if use_overlaps && (step_idx as usize) < overlaps_str.len() {
                overlaps_str[step_idx as usize] = "@";
            }

            if segment.is_empty() {
                continue;
            }

            let mut graph_segment_opt = gfa.find_segment_with_name(&segment);
            
            if graph_segment_opt.is_none() {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::SegmentNotFound,
                    segment.to_owned(),
                ));

                if options.handle_missing_segment == MissingSegmentOptions::HardSkip {
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::InvalidPath,
                        segment.to_owned(),
                    ));
                    return (None, errors);
                }

                if options.handle_missing_segment == MissingSegmentOptions::CreateGhost {
                    gfa.create_ghost_segment(segment.to_string());
                    graph_segment_opt = gfa.find_segment_with_name(&segment);
                }

                if options.handle_missing_segment == MissingSegmentOptions::SoftSkip {
                    continue;
                }
            }

            let graph_segment = graph_segment_opt.unwrap();
            let segment_line_no = graph_segment.line_no as u32;
            let curr_step_incoming_links = graph_segment.incoming_links.clone();

            let curr_step = Step {
                segment_id: segment_line_no,
                orientation,
            };

            // check to see if a link exists to connect them together
            if prev_step.is_none() {
                prev_step = Some(curr_step);
            } else {
                let prev_step_orientation = prev_step.clone().unwrap().orientation;

                let curr_step_segment_name = graph_segment.name.clone();
                let prev_step_segment = gfa
                    .find_segment_mut(prev_step.clone().unwrap().segment_id as usize)
                    .map(|s| s.name.clone());

                if prev_step_segment.is_none() {
                    continue; // already reported in previous iteration
                }

                let prev_step_segment_name = prev_step_segment.unwrap();

                let mut found_link_between_segments = false;
                let mut found_implicit_link_between_segments = false;

                for link_no in curr_step_incoming_links.iter() {
                    let link = gfa
                        .find_link_mut(*link_no)
                        .expect("incoming_links is managed by segment.rs");

                    if link.from_segment == prev_step_segment_name
                        && link.to_segment == curr_step_segment_name
                        && prev_step_orientation == link.from_orientation
                        && curr_step.orientation == link.to_orientation
                    {
                        // alles ist güt
                        found_link_between_segments = true;
                    }

                    if link.from_segment == prev_step_segment_name
                        && link.to_segment == curr_step_segment_name
                        && prev_step_orientation == !(link.to_orientation)
                        && curr_step.orientation == !(link.from_orientation)
                    {
                        // check if the user cares about these
                        found_implicit_link_between_segments = true;
                    }
                }

                if !found_link_between_segments && (found_implicit_link_between_segments && !options.allow_implicit_links) {
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::LinkNotFound,
                        format!(
                            "path step: {}{} -> {}{}",
                            prev_step_segment_name,
                            if prev_step_orientation { "+" } else { "-" },
                            curr_step_segment_name,
                            if curr_step.orientation { "+" } else { "-" }
                        ).to_string(),
                    ));
                }

                prev_step = Some(curr_step.clone());
            }

            // if we have a valid overlap, we store it
            // the @ marks it to be skipped when we process the overlaps
            if use_overlaps && (step_idx as usize) < overlaps_str.len() {
                overlaps_str[step_idx as usize] = real_overlap;
            }

            steps.push(Step {
                segment_id: segment_line_no,
                orientation,
            });
        }

        if use_overlaps {
            for (step_index, overlap) in overlaps_str.iter().enumerate() {
                // if we've decided to skip a step for some reason, skip the overlap as well
                if *overlap == "@" {
                    // set the last step's overlap to *, since it doesn't apply anymore
                    // now that the step was skipped
                    overlaps.pop();
                    overlaps.push("*".to_string());
                    continue;
                }

                if !is_valid_cigar(overlap) && *overlap != "*" {
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::InvalidCIGAR,
                        overlap.to_string(),
                    ));
                    overlaps.push("*".to_string());
                    continue;
                }

                // if overlap is *, we substitute it with the link overlap if possible
                if *overlap == "*" && options.substitute_path_overlaps {
                    if overlaps_str.get(step_index + 1) == Some(&"@") {
                        // if the next overlap is @, the segment might not exist
                        // skip this overlap just in case
                        break;
                    }

                    // if CIGAR is *, we use the link CIGAR
                    let step_segment_current_str = steps_str
                        .get(step_index)
                        .expect("overlaps_str.len should be 1 less than steps_str.len")
                        .trim_end_matches(['+', '-']);

                    let step_segment_current = &gfa
                        .find_segment_with_name(step_segment_current_str)
                        .expect("already checked segment exists");

                    let step_segment_next_str = steps_str
                        .get(step_index + 1)
                        .expect("overlaps_str.len should be 1 less than steps_str.len")
                        .trim_end_matches(['+', '-']);

                    let outgoing_links = step_segment_current.outgoing_links.clone();

                    let candidate_link_no = outgoing_links.iter().copied().find(|&link_no| {
                        let link = &gfa
                            .find_link_mut(link_no)
                            .expect("outgoing_links is managed by segment.rs");

                        link.to_segment == step_segment_next_str
                            && link.from_orientation == steps_str[step_index].ends_with("+")
                            && link.to_orientation == steps_str[step_index + 1].ends_with("+")
                    });

                    if candidate_link_no.is_none() {
                        break;
                    }

                    let link_to_next = &gfa.find_link_mut(candidate_link_no.unwrap());

                    if link_to_next.is_none() {
                        break;
                    }

                    overlaps.push(link_to_next.as_ref().unwrap().overlap.to_string());
                    continue;
                }

                // if cigar is valid, add it to the overlaps
                overlaps.push(overlap.to_string());
            }
        }

        (
            Some(Self {
                line_no: n,
                raw: raw.to_owned(),
                tags: map.clone(),

                name: name.to_string(),
                steps,
                overlaps,
            }),
            errors,
        )
    }

    pub fn to_raw_line(&self, _: GFAVersion, gfa: &GfaParser) -> String {
        self.to_raw_line_v1(gfa)
    }

    fn to_raw_line_v1(&self, gfa: &GfaParser) -> String {
        let name = self.name.as_str();
        let steps = self
            .steps
            .iter()
            .map(|s| {
                let seg_name = gfa
                    .find_segment(s.segment_id as usize)
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                format!("{}{}", seg_name, if s.orientation { "+" } else { "-" })
            })
            .collect::<Vec<String>>()
            .join(",");

            let overlaps = self.overlaps.join(",");

        build_gfa_line('P', &[name, &steps, &overlaps], &self.tags)
    }
}



#[cfg(test)]
mod tests {
    use crate::{errors::ParseMessageSeverity, gfa::{GfaParser, ParseOptions}};

    #[test]
    fn test_working_path() {
        let mut gfa = GfaParser::new();

        let _ = gfa.parse("test/path.gfa", &ParseOptions::default());
    
        gfa.messages.iter().for_each(|e| {
            e.print_formatted_error();
        });

        let has_errors = gfa.messages.iter().any(|e| e.severity() != ParseMessageSeverity::Warn && e.severity() != ParseMessageSeverity::Info);
        
        // TODO: write real test for path
        assert!(!has_errors);
    }
}