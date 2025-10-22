use crate::errors::ParseMessage;
use crate::errors::ParseMessageCode;
use crate::gfa::GFAVersion;
use crate::gfa::GfaParser;
use crate::gfa::ParseOptions;
use crate::line::bridge::parse_generic_bridge;
use crate::line::bridge::BridgeParts;
use crate::line::bridge::BridgeType;
use crate::line::utils::build_gfa_line;
use crate::optional_field::TagMap;

#[derive(Debug, Clone, Default)]
pub struct Containment {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub container: String,
    pub container_orientation: bool,
    pub contained: String,
    pub contained_orientation: bool,
    pub position: i32,
    pub overlap: String,
}

pub static REQ_COLUMNS_CONTAIN: usize = 7;

impl Containment {
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

        let (containment_as_bridge, errors) = parse_generic_bridge(
            gfa,
            BridgeParts{
                bridge_type: BridgeType::Containment,
                from_segment: parts[1],
                from_orientation: parts[2],
                to_segment: parts[3],
                to_orientation: parts[4],
                overlap: Some(parts[6]),
            },
            raw,
            n,
            map,
            options,
        );

        if containment_as_bridge.is_none() {
            return (None, errors);
        }

        let mut errors = errors;

        let containment = containment_as_bridge.unwrap();

        // check if position is a valid integer
        let position = match parts[5].parse() {
            Ok(p) => p,
            Err(_) => {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidPosition,
                    parts[5].to_owned(),
                ));
                0 // default to 0
            }
        };

        if let Some(container_segment) = gfa.find_segment_with_name(parts[1]) {
            if position < 0 || position > container_segment.get_length() {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidPosition,
                    parts[5].to_owned(),
                ));
            }
        }

        (
            Some(Self {
                line_no: n,
                raw: raw.to_owned(),
                tags: map.clone(),

                container: containment.from_segment,
                container_orientation: containment.from_orientation,
                contained: containment.to_segment,
                contained_orientation: containment.to_orientation,
                position,
                overlap: parts[6].to_owned(),
            }),
            errors,
        )
    }

    pub fn to_raw_line(&self, _: GFAVersion) -> String {
        self.to_raw_line_v1()
    }

    fn to_raw_line_v1(&self) -> String {
        build_gfa_line(
            'C',
            &[
                self.container.as_str(),
                if self.container_orientation { "+" } else { "-" },
                self.contained.as_str(),
                if self.contained_orientation { "+" } else { "-" },
                &self.position.to_string(),
                &self.overlap,
            ], 
            &self.tags
        )
    }
}

