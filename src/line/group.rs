use crate::errors::ParseMessage;
use crate::errors::ParseMessageCode;
use crate::gfa::GfaParser;
use crate::gfa::MissingSegmentOptions;
use crate::gfa::ParseOptions;

#[derive(Debug, Clone)]
pub struct GenericGroup {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GroupType {
    UnorderedGroup,
    OrderedGroup,
}

pub struct GroupParts<'a> {
    pub group_type: GroupType,
    pub name: &'a str,
    pub members: &'a str,
}

pub fn parse_generic_group(
    gfa: &mut GfaParser,
    parts: GroupParts<'_>,
    n: usize,
    options: &ParseOptions,
) -> (Option<GenericGroup>, Vec<ParseMessage>) {
        let mut errors = vec![];

        let group_type = parts.group_type;

        let name = if parts.name != "*" {
            &gfa.ensure_name_unique(n, parts.name.to_string())
        } else {
            let group_type_str = if matches!(group_type, GroupType::OrderedGroup) {
                "O"
            } else {
                "U"
            };

            let new_name = format!("anon_{group_type_str}_{n}");
            &gfa.ensure_name_unique(n, new_name)
        };

        let mut members = vec![];
        let members_str = parts.members.split(" ").collect::<Vec<&str>>();

        // check if every group member exists in the GFA file
        for member in members_str {
            let member_name = member.trim_end_matches(['+', '-']);
            let record_exists = gfa.is_name_in_namespace(member_name);

            if !record_exists {
                errors.push(ParseMessage::new(
                    n,
                    ParseMessageCode::GroupMemberNotFound,
                    member.to_owned(),
                ));

                if options.handle_missing_segment == MissingSegmentOptions::HardSkip {
                    errors.push(ParseMessage::new(
                        n,
                        ParseMessageCode::InvalidGroup,
                        member.to_owned(),
                    ));

                    return (None, errors);
                }
            }

            members.push(member.to_owned());
        }

        (
            Some(GenericGroup {
                name: name.to_string(),
                members,
            }),
            errors,
        )
}
