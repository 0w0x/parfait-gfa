use crate::errors::ParseMessage;
use crate::errors::ParseMessageCode;
use crate::gfa::GFAVersion;
use crate::gfa::GfaParser;
use crate::gfa::MissingSegmentOptions;
use crate::line::utils::build_gfa_line;
use crate::line::utils::parse_interval;
use crate::line::utils::Interval;
use crate::optional_field::TagMap;
use crate::line::utils::Alignment;
use crate::line::utils::DirectedReference;
use crate::line::utils::deduce_alignment;
use crate::line::utils::parse_directed_reference;

#[derive(Debug, Clone)]
pub struct Fragment {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub segment_name: String,
    pub external_name: DirectedReference,
    pub segment_interval: Interval,
    pub fragment_interval: Interval,
    pub alignment: Option<Alignment>,
}

pub static REQ_COLUMNS_FRAGMENT: usize = 8;

impl Fragment {
    pub fn parse_line(
        (gfa, parts, raw, n, map, options): (
            &mut GfaParser,
            &[&str],
            &str,
            usize,
            &mut TagMap,
            &crate::gfa::ParseOptions,
        ),
    ) -> (Option<Self>, Vec<ParseMessage>) {
        let mut errors = vec![];

        // check if segment exists
        let segment = gfa.find_segment_with_name(parts[1]);
        
        if segment.is_none() {
            errors.push(ParseMessage::new(
                n,
                ParseMessageCode::SegmentNotFound,
                parts[1].to_owned(),
            ));

            match options.handle_missing_segment {
                MissingSegmentOptions::Ignore => {}
                MissingSegmentOptions::HardSkip | MissingSegmentOptions::SoftSkip => {
                    return (None, errors);
                }
                MissingSegmentOptions::CreateGhost => {
                    // check to see if the missing name is valid
                    let _ = &gfa.create_ghost_segment(parts[1].to_owned());
                }
            }
        }

        // add the fragment to the segment
        if let Some(s) = gfa.find_segment_with_name(parts[1]) {
            s.fragments.push(n);
        }

        let referenced_segment = gfa.find_segment_with_name(parts[1]);        

        // check if external reference is valid
        let external = parse_directed_reference(parts[2]).unwrap_or_else(|mut e| {
            e.line = n;
            errors.push(e);

            // default to the reference name in forward ori
            DirectedReference {
                reference: parts[1].to_owned(),
                direction: true
            }
        });
        
        let segment_interval = parse_interval(n, &mut errors, referenced_segment.as_deref(), parts[3], parts[4]);
        let fragment_interval = parse_interval(n, &mut errors, None, parts[5], parts[6]);

        if segment_interval.is_err() || fragment_interval.is_err() {
            // if any of the fragment positions were invalid, don't even
            // bother trying to parse the record. any possible assumptions
            // would most likely be wrong at best, and misleading at worst

            // the only recovery i can think of is to assume it's a blunt end
            // i'll implement this if its requested. for now, skip the record

            return (None, errors);
        }

        let segment_interval = segment_interval.unwrap();
        let fragment_interval = fragment_interval.unwrap();

        // check if alignment is valid

        let alignment = deduce_alignment(parts[7]).unwrap_or_else(|mut e| {
            e.line = n;
            errors.push(e);
            None
        });

        (
            Some(Self {
                line_no: n,
                raw: raw.to_owned(),
                tags: map.clone(),

                segment_name: parts[1].to_owned(),
                external_name: external,
                segment_interval,
                fragment_interval,
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
        // fragments are a unique v2 concept, that can't be abstracted
        // to a bridge (since fragments exist outside of the gfa file)
        
        // maybe this could be another segment + a containment line?
        
        // if you have a use case for converting fragments to v1,
        // please open an issue about this
        "".to_string()
    }

    fn to_raw_line_v2(&self) -> String {
        build_gfa_line(
            'F',
            &[
                self.segment_name.as_str(),
                self.external_name.to_string().as_str(),
                &self.segment_interval.begin.to_string(),
                &self.segment_interval.end.to_string(),
                &self.fragment_interval.begin.to_string(),
                &self.fragment_interval.end.to_string(),
                &self.alignment.as_ref().map_or("*".to_string(), |a| a.to_string()),
            ], 
            &self.tags
        )
    }
}



#[cfg(test)]
mod tests {
    use crate::gfa::GfaParser;
    use crate::gfa::ParseOptions;

    #[test]
    fn test_fragment_parse_line() {
        let mut gfa = GfaParser::new();

        let _ = gfa.parse("test/fragment.gfa", &ParseOptions::default());

        for error in &gfa.messages {
            error.print_formatted_error();
        }

        // TODO: write real tests
        assert_eq!(gfa.messages.len(), 20);
    }
}
