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
pub struct Jump {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub from_segment: String,
    pub from_orientation: bool,
    pub to_segment: String,
    pub to_orientation: bool,
    pub distance: Option<i32>,
}

pub static REQ_COLUMNS_JUMP: usize = 6;

impl Jump {
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
        let (jump_as_bridge, errors) =
            parse_generic_bridge(gfa, BridgeParts{
                bridge_type: BridgeType::Jump,
                from_segment: parts[1],
                from_orientation: parts[2],
                to_segment: parts[3],
                to_orientation: parts[4],
                overlap: None,
         }, raw, n, map, options);

        if jump_as_bridge.is_none() {
            return (None, errors);
        }

        let mut errors = errors;

        let jump = jump_as_bridge.unwrap();

        if let Some(sc) = map.get::<i32>("SC") {
            if sc != 0 && sc != 1 {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidShortcut,
                    sc.to_string(),
                ));
            }
        }

        let distance = match parts[5] {
            "*" => None,
            s => s
                .parse::<i32>()
                .map(Some)
                .unwrap_or_else(|_| {
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::InvalidJumpDistance,
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

                from_segment: jump.from_segment,
                from_orientation: jump.from_orientation,
                to_segment: jump.to_segment,
                to_orientation: jump.to_orientation,
                distance,
            }),
            errors,
        )
    }

    pub fn to_raw_line(&self, _: GFAVersion) -> String {
        self.to_raw_line_v1()
    }

    fn to_raw_line_v1(&self) -> String {
        let columns = [
            self.from_segment.as_str(),
            if self.from_orientation { "+" } else { "-" },
            self.to_segment.as_str(),
            if self.to_orientation { "+" } else { "-" },
            &self.distance.map_or("*".to_string(), |d| d.to_string()),
        ];

        // build the GFA line
        build_gfa_line(
            'J',
            &columns,
            &self.tags,
        )
    }

}

