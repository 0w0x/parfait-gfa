use crate::gfa::GFAVersion;
use crate::gfa::ParseOptions;
use crate::line::record::GfaRecord;
use crate::line::utils::build_gfa_line;
use crate::optional_field::OptionalFieldValue;
use crate::errors::ParseMessageCode;
use crate::errors::ParseMessage;
use crate::gfa::GfaParser;
use crate::optional_field::TagMap;

#[derive(Debug, Clone, Default)]
pub struct Header {
    pub line_no: usize,
    pub raw: String,
    pub tags: TagMap,

    pub version: Option<String>,
}

pub static REQ_COLUMNS_HEADER: usize = 1; // technically the version tag is not a required column

impl Header {
    pub fn new() -> Self {
        Self {
            line_no: 0,
            raw: String::new(),
            tags: TagMap::default(),
            version: "1.0".to_string().into(),
        }
    }

    pub fn parse_line(
        (gfa, _, raw, n, map , _): (&mut GfaParser, &[&str], &str, usize, &mut TagMap, &ParseOptions),
    ) -> (Option<Self>, Vec<ParseMessage>) {
        let mut errors = vec![];

        let is_duplicate_header = gfa
            .records
            .iter()
            .any(|r| matches!(r, GfaRecord::Header(_)));

        if is_duplicate_header {
            errors.push(ParseMessage::new(
                n,
                ParseMessageCode::DuplicateHeader,
                raw.to_owned(),
            ));
        }

        let valid_versions = ["1", "1.0", "1.1", "1.2", "2", "2.0"];

        // if VN tag exists, check if it's valid
        if map.contains("VN")
            && !map
                .get::<String>("VN")
                .map_or(false, |v| valid_versions.contains(&v.as_str()))
        {
            errors.push(ParseMessage::new(
                n,
                ParseMessageCode::UnknownVersion,
                raw.to_owned(),
            ));

            // default VN to 1.0
            map.0.insert(
                "VN".to_string(),
                OptionalFieldValue::String("1.0".to_string()),
            );
        }

        if map.get::<String>("VN").is_none() {
            // check if VN tag is present, all GFA files need a version tag
            // (actually in V2, they're optional, but nobody needs to know that)
            errors.push(ParseMessage::new(
                n,
                ParseMessageCode::MissingVersionTag,
                raw.to_owned(),
            ));

            // default to 1.0 if VN is missing
            // TODO: infer version from file instead of defaulting
            map.0.insert(
                "VN".to_string(),
                OptionalFieldValue::String("1.0".to_string()),
            );
        }

        if !is_duplicate_header {
            gfa.version = map
                .get::<String>("VN")
                .expect("already ensured VN tag")
                .into();

            if map.contains("TS") {
                gfa.trace = map.get::<String>("TS");
            }
        }

        (
            Some(Self {
                line_no: n,
                raw: raw.to_owned(),
                tags: map.clone(),
                version: map.get("VN"),
            }),
            errors,
        )
    }

    pub fn to_raw_line(&self, version: GFAVersion) -> String {
        self.to_raw_line_v1(version)
    }

    fn to_raw_line_v1(&self, version: GFAVersion) -> String {
        let mut tag_clone: TagMap = self.tags.clone();
        
        tag_clone.0.insert(
            "VN".to_string(),
            OptionalFieldValue::String(version.to_string()),
        );

        build_gfa_line(
            'H',
            &[],
            &tag_clone,
        )
    }
}

