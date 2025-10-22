use crate::errors::ParseMessage;
use crate::gfa::GFAVersion;
use crate::gfa::GfaParser;
use crate::gfa::ParseOptions;
use crate::line::group::parse_generic_group;
use crate::line::group::GroupParts;
use crate::line::group::GroupType;
use crate::line::utils::build_gfa_line;
use crate::optional_field::TagMap;

#[derive(Debug, Clone, Default)]
pub struct OrderedGroup {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub name: String,
    pub members: Vec<String>,
}

pub static REQ_COLUMNS_ORDERED: usize = 3;

impl OrderedGroup {
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
        let (ordered_as_group, errors) = parse_generic_group(
            gfa,
            GroupParts {
                group_type: GroupType::OrderedGroup,
                name: parts[1],
                members: parts[2],
            },
            n,
            options,
        );

        if ordered_as_group.is_none() {
            return (None, errors);
        }

        let ordered = ordered_as_group.unwrap();

        (
            Some(Self {
                line_no: n,
                raw: raw.to_owned(),
                tags: map.clone(),

                name: ordered.name,
                members: ordered.members,
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
        // TODO: convert ordered groups to v1 paths
        "".to_string()
    }

    fn to_raw_line_v2(&self) -> String {
        let members_str = self.members.join(" ");
        let parts = vec![self.name.as_str(), members_str.as_str()];
        build_gfa_line('U', &parts, &self.tags)
    }
}
