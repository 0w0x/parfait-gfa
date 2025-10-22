use crate::errors::ParseMessage;
use crate::errors::ParseMessageCode;
use crate::gfa::GFAVersion;
use crate::gfa::GfaParser;
use crate::gfa::ParseOptions;
use crate::line::bridge::parse_generic_bridge;
use crate::line::bridge::BridgeParts;
use crate::line::bridge::BridgeType;
use crate::line::utils::DirectedReference;
use crate::line::utils::build_gfa_line;
use crate::line::utils::is_valid_name;
use crate::line::utils::parse_directed_reference;
use crate::optional_field::OptionalFieldValue;
use crate::optional_field::TagMap;

#[derive(Debug, Clone)]
pub struct Gap {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub id: Option<String>,
    pub from: DirectedReference,
    pub to: DirectedReference,
    pub distance: i32,
    pub variance: Option<i32>,
}

pub static REQ_COLUMNS_GAP: usize = 6;

impl Gap {
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

        let (gap_as_bridge, errors) = parse_generic_bridge(
            gfa,
            BridgeParts {
                bridge_type: BridgeType::Gap,
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

        // TODO: this is a copy/paste of edge.rs
        // this should probably be refactored

        if gap_as_bridge.is_none() {
            return (None, errors);
        }

        let mut errors = errors;

        let gap = gap_as_bridge.unwrap();

        let from = DirectedReference {
            reference: gap.from_segment,
            direction: gap.from_orientation,
        };

        let to = DirectedReference {
            reference: gap.to_segment,
            direction: gap.to_orientation,
        };

        let mut gap_id;

        if map.contains("ID") {
            if parts[1] != "*" {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::RedundantEdgeIDTag,
                    map.get::<String>("ID").unwrap().to_owned(),
                ));

                gap_id = Some(parts[1].to_owned());
            } else {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::EdgeIDTagUsedInAnonEdge,
                    map.get::<String>("ID").unwrap().to_owned(),
                ));
                gap_id = map.get::<String>("ID");
            }
        } else {
            gap_id = if parts[1] == "*" {
                None
            } else {
                Some(parts[1].to_owned())
            };
        }

        gap_id = gap_id
            .and_then(|id| {
                if is_valid_name(&id) {
                    return Some(id);
                }
                errors.push(ParseMessage::new(n, ParseMessageCode::InvalidID, id.to_owned()));
                None
            })
            .and_then(|id| Some(gfa.ensure_name_unique(n, id.clone())))
            .and_then(|id| Some(id.clone()));

        let distance = parts[4]
            .parse::<i32>()
            .map_err(|_| {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidGapDistance,
                    parts[4].to_string(),
                ));
            })
            .unwrap_or(0);

        let variance = match parts[5] {
            "*" => None,
            s => s.parse::<i32>().map(Some).unwrap_or_else(|_| {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidVariance,
                    s.to_string(),
                ));
                None
            }),
        };

        (
            Some(Self {
                line_no: n,
                raw: raw.to_owned(),
                tags: map.clone(),

                id: gap_id,
                from,
                to,
                distance,
                variance,
            }),
            errors,
        )
    }

    pub fn to_raw_line(&self, version: GFAVersion) -> String {
        match version {
            GFAVersion::V2 => self.to_raw_line_v2(),
            GFAVersion::V1_2 => self.to_raw_line_v1(false),
            _ => self.to_raw_line_v1(true),
        }
    }

    fn to_raw_line_v1(&self, is_v1_0: bool) -> String {
        let mut new_tags = self.tags.clone();

        if self.id.is_some() && !new_tags.contains("ID") {
            new_tags.0.insert(
                "ID".to_string(),
                OptionalFieldValue::String(self.id.clone().unwrap()),
            );
        }

        if self.variance.is_some() && !new_tags.contains("VA") {
            new_tags.0.insert(
                "VA".to_string(),
                OptionalFieldValue::Int(self.variance.unwrap()),
            );
        }

        // this link/jump was a gap in another life
        new_tags.add_flag("gap");

        // jumps only exist in v1.2, use a link for v1.0
        let record_type = if is_v1_0 { 'L' } else { 'J' };
        let fifth_column = if is_v1_0 {
            if !new_tags.contains("VA") {
                new_tags.0.insert(
                    "DI".to_string(),
                    OptionalFieldValue::Int(self.distance),
                );
            }

            "*".to_string()
        } else {
            self.distance.to_string()
        };

        let columns = [
            self.from.reference.as_str(),
            if self.from.direction { "+" } else { "-" },
            self.to.reference.as_str(),
            if self.to.direction { "+" } else { "-" },
            fifth_column.as_str(),
        ];

        build_gfa_line(record_type, &columns, &new_tags)
    }

    fn to_raw_line_v2(&self) -> String {
        build_gfa_line(
            'G',
            &[
                self.id.as_deref().unwrap_or("*"),
                self.from.to_string().as_str(),
                self.to.to_string().as_str(),
                self.distance.to_string().as_str(),
                &self.variance.map_or("*".to_string(), |v| v.to_string()),
            ],
            &self.tags,
        )
    }
}

