use crate::errors::ParseMessageCode;
use crate::gfa::GFAVersion;
use crate::gfa::ParseOptions;
use crate::errors::ParseMessage;
use crate::gfa::GfaParser;
use crate::line::bridge::parse_generic_bridge;
use crate::line::bridge::BridgeParts;
use crate::line::bridge::BridgeType;
use crate::line::utils::build_gfa_line;
use crate::line::utils::Alignment;
use crate::line::utils::DirectedReference;
use crate::line::utils::Interval;
use crate::line::utils::deduce_alignment;
use crate::line::utils::is_valid_name;
use crate::line::utils::parse_directed_reference;
use crate::line::utils::parse_interval;
use crate::optional_field::OptionalFieldValue;
use crate::optional_field::TagMap;

#[derive(Debug, Clone, Default)]
pub struct Edge {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub id: Option<String>,
    pub from: DirectedReference,
    pub to: DirectedReference,
    pub from_interval: Interval,
    pub to_interval: Interval,
    pub alignment: Option<Alignment>,
}

pub static REQ_COLUMNS_EDGE: usize = 9;

impl Edge {
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
        let from = match parse_directed_reference(parts[2]) {
            Ok(f) => f,
            Err(mut e) => {
                e.line = n;
                return (None, vec![e]);
            }
        };

        let to = match parse_directed_reference(parts[3]) {
            Ok(t) => t,
            Err(mut e) => {
                e.line = n;
                return (None, vec![e]);
            }
        };

        let (edge_as_bridge, errors) = parse_generic_bridge(
            gfa,
            BridgeParts {
                bridge_type: BridgeType::Edge,
                from_segment: &from.reference,
                from_orientation: if from.direction { "+" } else { "-" },
                to_segment: &to.reference,
                to_orientation: if to.direction { "+" } else { "-" },
                overlap: None,
            },
            raw,
            n,
            map,
            options,
        );

        if edge_as_bridge.is_none() {
            return (None, errors);
        }

        let mut errors = errors;

        let edge = edge_as_bridge.unwrap();

        let from = DirectedReference {
            reference: edge.from_segment,
            direction: edge.from_orientation,
        };

        let to = DirectedReference {
            reference: edge.to_segment,
            direction: edge.to_orientation,
        };

        let mut edge_id;

        if map.contains("ID") {
            if parts[1] != "*" {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::RedundantEdgeIDTag,
                    map.get::<String>("ID").unwrap().to_owned(),
                ));

                edge_id = Some(parts[1].to_owned());
            } else {
                // very silly scenario that's easy enough to handle
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::EdgeIDTagUsedInAnonEdge,
                    map.get::<String>("ID").unwrap().to_owned(),
                ));
                edge_id = map.get::<String>("ID");
            }
        } else {
            edge_id = if parts[1] == "*" {
                None
            } else {
                Some(parts[1].to_owned())
            };
        }

        edge_id = edge_id
            .and_then(|id| {
                if is_valid_name(&id) {
                    return Some(id);
                }
                errors.push(ParseMessage::new(n, ParseMessageCode::InvalidID, id.to_owned()));
                None
            })
            .and_then(|id| Some(gfa.ensure_name_unique(n, id.clone())))
            .and_then(|id| Some(id.clone()));

        let from_segment = gfa.find_segment_with_name(&from.reference);

        let from_interval =
            parse_interval(n, &mut errors, from_segment.as_deref(), parts[4], parts[5]);

        let to_segment = gfa.find_segment_with_name(&to.reference);

        let to_interval = parse_interval(n, &mut errors, to_segment.as_deref(), parts[6], parts[7]);

        if from_interval.is_err() || to_interval.is_err() {
            // same policy as fragment for now, skip the record if any of the positions were invalid
            return (None, errors);
        }

        let from_interval = from_interval.unwrap();
        let to_interval = to_interval.unwrap();

        let alignment = deduce_alignment(parts[8]).unwrap_or_else(|mut e| {
            e.line = n;
            errors.push(e);
            None
        });

        (
            Some(Self {
                line_no: n,
                raw: raw.to_owned(),
                tags: map.clone(),

                id: edge_id,
                from,
                to,
                from_interval,
                to_interval,
                alignment,
            }),
            errors,
        )
    }

    pub fn to_raw_line(&self, version: GFAVersion) -> String {
        match version {
            GFAVersion::V2 => self.to_raw_line_v2(),
            _ => self.to_raw_line_v1(),
        }
    }

    fn to_raw_line_v1(&self) -> String {
        let mut new_tags = self.tags.clone();

        if self.id.is_some() && !new_tags.contains("ID") {
            new_tags.0.insert(
                "ID".to_string(),
                OptionalFieldValue::String(self.id.clone().unwrap()),
            );
        }

        let overlap = if self.alignment.is_some() {
            match self.alignment.as_ref().unwrap() {
                Alignment::Trace(trace) => {
                    new_tags.0.insert(
                        "TS".to_string(),
                        OptionalFieldValue::String(trace.to_string()),
                    );
                    "*".to_string()
                }
                Alignment::CIGAR(cigar) => {
                    cigar.to_string()
                }
            }
        } else {
            "*".to_string()
        };

        // this link was an edge in another life
        new_tags.add_flag("edge");

        // jumps only exist in v1.2, use a link for v1.0
        let columns = [
            self.from.reference.as_str(),
            if self.from.direction { "+" } else { "-"},
            self.to.reference.as_str(),
            if self.to.direction { "+" } else { "-" },
            overlap.as_str(),
        ];

        // converting to a link loses the alignment information
        // might be worth revisiting this in the future
        // to see if i can preserve it with containments/etc.

        build_gfa_line('L', &columns, &new_tags)
    }

    fn to_raw_line_v2(&self) -> String {
        build_gfa_line(
            'E',
            &[
                self.id.as_deref().unwrap_or("*"),
                self.from.to_string().as_str(),
                self.to.to_string().as_str(),
                &self.from_interval.begin.to_string(),
                &self.from_interval.end.to_string(),
                &self.to_interval.begin.to_string(),
                &self.to_interval.end.to_string(),
                &self.alignment.as_ref().map_or("*".to_string(), |a| a.to_string()),
            ], 
            &self.tags
        )
    }

}

