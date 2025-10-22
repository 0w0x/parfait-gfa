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
pub struct UnorderedGroup {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub name: String,
    pub members: Vec<String>,
}

pub static REQ_COLUMNS_UNORDERED: usize = 3;

impl UnorderedGroup {
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
        let (unordered_as_group, errors) = parse_generic_group(
            gfa,
            GroupParts {
                group_type: GroupType::UnorderedGroup,
                name: parts[1],
                members: parts[2],
            },
            n,
            options,
        );

        if unordered_as_group.is_none() {
            return (None, errors);
        }

        let unordered = unordered_as_group.unwrap();

        (
            Some(Self {
                line_no: n,
                raw: raw.to_owned(),
                tags: map.clone(),

                name: unordered.name,
                members: unordered.members,
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
        // TODO: support unordered groups in v1
        "".to_string()
    }

    fn to_raw_line_v2(&self) -> String {
        let members_str = self.members.join(" ");
        let parts = vec![self.name.as_str(), members_str.as_str()];
        
        build_gfa_line('U', &parts, &self.tags)
    }

    pub fn derive_group(&self, _: &GfaParser) -> Vec<String> {
        let members = vec![];

        // TODO: implement unordered group expansion
        // logic is basically just:
        // - store all members in a set
        // - iterate over the set and for each member, check against all the other members
        //   - find everything in between the two members
        //   - i.e. two segments is everything in between them

        // my naive implementation would be O(forever) so i'm not going to bother until someone requests it

        members
    }
}
