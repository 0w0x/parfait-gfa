use crate::errors::ParseMessage;
use crate::errors::ParseMessageCode;
use crate::gfa::GfaParser;
use crate::gfa::MissingSegmentOptions;
use crate::gfa::ParseOptions;
use crate::line::utils::is_valid_cigar;
use crate::line::utils::is_valid_name;
use crate::optional_field::OptionalFieldValue;
use crate::optional_field::TagMap;

#[derive(Debug, Clone)]
pub struct GenericBridge {
    pub from_segment: String,
    pub from_orientation: bool,
    pub to_segment: String,
    pub to_orientation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BridgeType {
    Jump,
    Link,
    Containment,
    Edge,
    Gap,
}

pub struct BridgeParts<'a> {
    pub bridge_type: BridgeType,
    pub from_segment: &'a str,
    pub from_orientation: &'a str,
    pub to_segment: &'a str,
    pub to_orientation: &'a str,
    pub overlap: Option<&'a str>,
}

pub fn parse_generic_bridge(
    gfa: &mut GfaParser,
    parts: BridgeParts<'_>,
    raw: &str,
    n: usize,
    map: &mut TagMap,
    options: &ParseOptions,
) -> (Option<GenericBridge>, Vec<ParseMessage>) {
        let mut errors = vec![];

        let bridge_type = parts.bridge_type;

        let mut from_segment = parts.from_segment.to_owned();
        let mut to_segment = parts.to_segment.to_owned();

        // check if the segment exists
        let p_from_segment_none = gfa.find_segment_with_name(&from_segment).is_none();
        let p_to_segment_none = gfa.find_segment_with_name(&to_segment).is_none();

        if p_from_segment_none {
            errors.push(ParseMessage::new(
                n,
                ParseMessageCode::SegmentNotFound,
                from_segment.to_owned(),
            ));
        }

        if p_to_segment_none {
            errors.push(ParseMessage::new(
                n,
                ParseMessageCode::SegmentNotFound,
                to_segment.to_owned(),
            ));
        }

        
        if p_from_segment_none || p_to_segment_none {
            if p_from_segment_none && p_to_segment_none {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::BridgeGoesNowhere,
                    format!("({:?}) / {} and {}", parts.bridge_type, from_segment, to_segment),
                ));
            }

            if options.handle_missing_segment != MissingSegmentOptions::Ignore {
                // Skip upon a missing segment if we have to
                if options.handle_missing_segment != MissingSegmentOptions::CreateGhost {
                    return (None, errors);
                }

                // Otherwise, create a ghost segment

                if p_from_segment_none {
                    let g = &gfa.create_ghost_segment(from_segment.to_owned());
                    from_segment = g.name.clone();
                }

                if p_to_segment_none {
                    let g = &gfa.create_ghost_segment(to_segment.to_owned());
                    to_segment = g.name.clone();
                }
            }
        }

        if let Some(from) = gfa.find_segment_with_name(&from_segment) {
            match bridge_type {
                BridgeType::Link => from.outgoing_links.push(n),
                BridgeType::Jump => from.outgoing_jumps.push(n),
                BridgeType::Containment => from.containments.push(n),
                BridgeType::Edge => from.outgoing_edges.push(n),
                BridgeType::Gap => from.outgoing_gaps.push(n),
            }
        }

        if let Some(to) = gfa.find_segment_with_name(&to_segment) {
            match bridge_type {
                BridgeType::Link => to.incoming_links.push(n),
                BridgeType::Jump => to.incoming_jumps.push(n),
                BridgeType::Containment => to.contained_by.push(n),
                BridgeType::Edge => to.incoming_edges.push(n),
                BridgeType::Gap => to.incoming_gaps.push(n),
            }
        }

        // check if the orientations are valid
        if parts.from_orientation != "-" && parts.from_orientation != "+" {
            errors.push(ParseMessage::new(
                n,
                ParseMessageCode::InvalidOrientation,
                parts.from_orientation.to_owned(),
            ));
        }

        if parts.to_orientation != "-" && parts.to_orientation != "+" {
            errors.push(ParseMessage::new(
                n,
                ParseMessageCode::InvalidOrientation,
                parts.to_orientation.to_owned(),
            ));
        }

        // default to + if orientation is not valid
        let from_orientation = parts.from_orientation != "-";
        let to_orientation = parts.to_orientation != "-";

        if from_segment == to_segment {
            let code = match bridge_type {
                BridgeType::Link => ParseMessageCode::SelfBridge,
                BridgeType::Jump => ParseMessageCode::SelfBridge,
                BridgeType::Edge => ParseMessageCode::SelfBridge,
                BridgeType::Containment => ParseMessageCode::SelfContainment,
                BridgeType::Gap => ParseMessageCode::SelfBridge,
            };

            errors.push(ParseMessage::new(
                n,
                code,
                raw.to_owned(),
            ));            
        }

        // Add Link/Jump/Containment EdgeID tag to namespace
        if !matches!(parts.bridge_type, BridgeType::Edge | BridgeType::Gap) {
            if let Some(edge_id) = map.get::<String>("ID") {
                if !is_valid_name(&edge_id) {
                    gfa.messages.push(ParseMessage::new(
                        n,
                        ParseMessageCode::InvalidID,
                        edge_id,
                    ));
                } else {
                    let uid = gfa.ensure_name_unique(n, edge_id);
                    map.0.insert("ID".into(), OptionalFieldValue::String(uid));
                }
            }
        }

        if let Some(overlap) = parts.overlap {    
            if overlap != "*" && !is_valid_cigar(overlap) {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidCIGAR,
                    overlap.to_owned(),
                ));
            }
        }

        (
            Some(GenericBridge {
                from_segment,
                from_orientation,
                to_segment,
                to_orientation,
            }),
            errors,
        )

}
