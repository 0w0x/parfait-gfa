use crate::gfa::GFAVersion;
use crate::line::containment::Containment;
use crate::line::edge::Edge;
use crate::line::fragment::Fragment;
use crate::line::gap::Gap;
use crate::line::header::Header;
use crate::line::jump::Jump;
use crate::line::link::Link;
use crate::line::ordered::OrderedGroup;
use crate::line::path::Path;
use crate::line::segment::Segment;
use crate::line::unordered::UnorderedGroup;
use crate::line::walk::Walk;
use crate::optional_field::TagMap;
use crate::errors::ParseMessageCode;
use crate::errors::ParseMessage;
use crate::gfa::GfaParser;
use crate::optional_field::collect_optional_fields;
use crate::parse_case;
use crate::record_accessors;

#[derive(Debug, Clone)]
pub enum GfaRecord {
    Header(Header),
    Segment(Segment),
    Link(Link),
    Containment(Containment),
    Path(Path),
    Walk(Walk),
    Jump(Jump),
    Fragment(Fragment),
    Edge(Edge),
    Gap(Gap),
    OrderedGroup(OrderedGroup),
    UnorderedGroup(UnorderedGroup),
}

impl GfaRecord {
    pub fn parse_line(
        (gfa, line, n, options): (&mut GfaParser, &str, usize, &crate::gfa::ParseOptions),
    ) -> (Option<Self>, Vec<ParseMessage>) {
        let parts: Vec<&str> = line.split('\t').collect();
        let record_type = parts.first().cloned();

        // keeping the raw lines is really only useful for debugging
        let raw = if options.store_raw_lines {
            line.to_owned()
        } else {
            "".to_string()
        };

        // get required columns based on the record type
        let required_columns = match record_type {
            Some("H") => crate::line::header::REQ_COLUMNS_HEADER,
            Some("S") => {
                if gfa.version == GFAVersion::V2 {
                    4
                } else {
                    3
                }
            }
            Some("L") => crate::line::link::REQ_COLUMNS_LINK,
            Some("C") => crate::line::containment::REQ_COLUMNS_CONTAIN,
            Some("P") => crate::line::path::REQ_COLUMNS_PATH,
            Some("W") => crate::line::walk::REQ_COLUMNS_WALK,
            Some("J") => crate::line::jump::REQ_COLUMNS_JUMP,
            Some("F") => crate::line::fragment::REQ_COLUMNS_FRAGMENT,
            Some("E") => crate::line::edge::REQ_COLUMNS_EDGE,
            Some("G") => crate::line::gap::REQ_COLUMNS_GAP,
            Some("O") => crate::line::ordered::REQ_COLUMNS_ORDERED,
            Some("U") => crate::line::unordered::REQ_COLUMNS_UNORDERED,
            _ => panic!("unreachable")
        };

        // check if there are enough columns
        if parts.len() < required_columns {
            return (
                None,
                vec![ParseMessage::new(
                    n,
                    ParseMessageCode::InvalidLine,
                    raw.to_owned(),
                )],
            );
        };

        let mut errors = vec![];

        // collect optional fields
        let (tags, tag_errs) = collect_optional_fields(
            n,
            record_type.expect("should have already skipped line if unknown record type"),
            &parts[required_columns..],
        );

        if let Some(err) = tag_errs.into_iter().next() {
            errors.push(err);
        }

        // keep note of all tag names encountered
        for tag in tags.iter() {
            gfa.tag_names.insert(tag.tag.clone());
        }

        let mut tag_map: TagMap = TagMap::from_vec(tags);

        let args = (
            gfa, 
            parts.as_slice(), 
            raw.as_str(), 
            n, 
            &mut tag_map, 
            options
        );

        let (record, mut record_errors) = match record_type {
            Some("H") => parse_case!(Header, Header, args),
            Some("S") => parse_case!(Segment, Segment, args),
            Some("L") => parse_case!(Link, Link, args),
            Some("C") => parse_case!(Containment, Containment, args),
            Some("P") => parse_case!(Path, Path, args),
            Some("W") => parse_case!(Walk, Walk, args),
            Some("J") => parse_case!(Jump, Jump, args),
            Some("F") => parse_case!(Fragment, Fragment, args),
            Some("E") => parse_case!(Edge, Edge, args),
            Some("G") => parse_case!(Gap, Gap, args),
            Some("O") => parse_case!(OrderedGroup, OrderedGroup, args),
            Some("U") => parse_case!(UnorderedGroup, UnorderedGroup, args),
            _ => panic!("unreachable"),
        };

        // add optional field errors
        record_errors.extend(errors);

        (record, record_errors)
    }

    // TODO: make this a macro if it gets too unwieldy

    pub fn line_no(&self) -> usize {
        match self {
            GfaRecord::Header(r) => r.line_no,
            GfaRecord::Segment(r) => r.line_no,
            GfaRecord::Link(r) => r.line_no,
            GfaRecord::Containment(r) => r.line_no,
            GfaRecord::Path(r) => r.line_no,
            GfaRecord::Walk(r) => r.line_no,
            GfaRecord::Jump(r) => r.line_no,
            GfaRecord::Fragment(r) => r.line_no,
            GfaRecord::Edge(r) => r.line_no,
            GfaRecord::Gap(r) => r.line_no,
            GfaRecord::OrderedGroup(r) => r.line_no,
            GfaRecord::UnorderedGroup(r) => r.line_no,
        }
    }

    pub fn to_raw_line(&self, version: GFAVersion, gfa: &GfaParser) -> String {
        match self {
            GfaRecord::Header(r) => r.to_raw_line(version),
            GfaRecord::Segment(r) => r.to_raw_line(version),
            GfaRecord::Link(r) => r.to_raw_line(version),
            GfaRecord::Containment(r) => r.to_raw_line(version),
            GfaRecord::Path(r) => r.to_raw_line(version, gfa),
            GfaRecord::Walk(r) => r.to_raw_line(version, gfa),
            GfaRecord::Jump(r) => r.to_raw_line(version),
            GfaRecord::Fragment(r) => r.to_raw_line(version),
            GfaRecord::Edge(r) => r.to_raw_line(version),
            GfaRecord::Gap(r) => r.to_raw_line(version),
            GfaRecord::OrderedGroup(r) => r.to_raw_line(version),
            GfaRecord::UnorderedGroup(r) => r.to_raw_line(version),
        }
    }
}

record_accessors! {
   impl GfaRecord {
        Header(Header) => (as_header, as_mut_header);
        Segment(Segment) => (as_segment, as_mut_segment);
        Link(Link) => (as_link, as_mut_link);
        Containment(Containment) => (as_containment, as_mut_containment);
        Path(Path) => (as_path, as_mut_path);
        Walk(Walk) => (as_walk, as_mut_walk);
        Jump(Jump) => (as_jump, as_mut_jump);
        Fragment(Fragment) => (as_fragment, as_mut_fragment);
        Edge(Edge) => (as_edge, as_mut_edge);
        Gap(Gap) => (as_gap, as_mut_gap);
        OrderedGroup(OrderedGroup) => (as_ordered_group, as_mut_ordered_group);
        UnorderedGroup(UnorderedGroup) => (as_unordered_group, as_mut_unordered_group);
    }
}