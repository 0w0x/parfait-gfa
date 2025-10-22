use crate::errors::ParseMessage;
use crate::gfa::GFAVersion;
use crate::gfa::GfaParser;
use crate::gfa::ParseOptions;
use crate::line::bridge::parse_generic_bridge;
use crate::line::bridge::BridgeParts;
use crate::line::bridge::BridgeType;
use crate::line::utils::build_gfa_line;
use crate::optional_field::TagMap;

#[derive(Debug, Clone)]
pub struct Link {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub from_segment: String,
    pub from_orientation: bool,
    pub to_segment: String,
    pub to_orientation: bool,
    pub overlap: String,
}


impl Default for Link {
    fn default() -> Self {
        Self {
            line_no: 0,
            raw: "".to_string(),
            tags: TagMap::new(),

            from_segment: "".to_string(),
            from_orientation: true,
            to_segment: "".to_string(),
            to_orientation: true,
            overlap: "*".to_string(),
        }
    }
}

pub static REQ_COLUMNS_LINK: usize = 6;

impl Link {
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
        let (link_as_bridge, errors) =
            parse_generic_bridge(gfa, BridgeParts{
                bridge_type: BridgeType::Link,
                from_segment: parts[1],
                from_orientation: parts[2],
                to_segment: parts[3],
                to_orientation: parts[4],
                overlap: Some(parts[5]),
        }, raw, n, map, options);

        if link_as_bridge.is_none() {
            return (None, errors);
        }

        let link = link_as_bridge.unwrap();

        (
            Some(Self {
                line_no: n,
                raw: raw.to_owned(),
                tags: map.clone(),

                from_segment: link.from_segment,
                from_orientation: link.from_orientation,
                to_segment: link.to_segment,
                to_orientation: link.to_orientation,
                overlap: parts[5].to_owned(),
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
            self.overlap.as_str(),
        ];

        // build the GFA line
        build_gfa_line(
            'L',
            &columns,
            &self.tags,
        )
    }
}

