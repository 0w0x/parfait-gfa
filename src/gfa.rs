use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

use clap::ValueEnum;

use crate::{
    errors::{ParseMessage, ParseMessageCode, ParseMessageSeverity},
    impl_enum_find_accessors,
    line::{
        containment::Containment, edge::Edge, fragment::Fragment, gap::Gap, header::Header,
        jump::Jump, link::Link, ordered::OrderedGroup, path::Path, record::GfaRecord,
        segment::Segment, unordered::UnorderedGroup, walk::Walk,
    },
};

#[derive(Debug, Default)]
pub struct GfaParser {
    pub records: Vec<GfaRecord>,
    pub messages: Vec<ParseMessage>,
    pub tag_names: HashSet<String>,
    pub version: GFAVersion,
    pub trace: Option<String>,

    namespace: HashMap<String, u32>,
    records_index: HashMap<usize, usize>,
    namespace_index: HashMap<String, usize>,
    max_lines: usize,
}

impl GfaParser {
    /// Creates a new GFA parser
    pub fn new() -> Self {
        Self {
            namespace_index: HashMap::new(),
            tag_names: HashSet::new(),
            records_index: HashMap::new(),
            ..Self::default()
        }
    }

    /// Parses the GFA file at the given path with [`ParseOptions`].
    ///
    /// The parsed records are stored in [`GfaParser::records`], and any errors encountered during parsing
    /// are stored in [`GfaParser::messages`].
    /// If any fatal errors are encountered, they are cloned and returned as a [`Err`].
    ///
    /// Example:
    /// ```
    /// use parfait_gfa::gfa::{GfaParser, ParseOptions};
    ///
    /// let mut parser = GfaParser::new();
    /// let result = parser.parse("path/to/file.gfa", &ParseOptions::default());
    ///
    /// match result {
    ///   Ok(_) => println!("Parsed successfully"),
    ///   Err(errors) => println!("Failed to parse file"),
    /// }
    /// ```
    pub fn parse(
        &mut self,
        path: impl Into<PathBuf>,
        options: &ParseOptions,
    ) -> Result<(), Vec<ParseMessage>> {
        let path_buf = path.into();

        // dont run on a directory
        if path_buf.is_dir() {
            self.messages.push(ParseMessage::new(
                0,
                ParseMessageCode::DirectoryError,
                path_buf.to_string_lossy().to_string(),
            ));
            return Err(self.messages.clone());
        }

        let file = match File::open(&path_buf) {
            Ok(f) => BufReader::new(f),
            Err(_) => {
                self.messages.push(ParseMessage::new(
                    0,
                    ParseMessageCode::IOError,
                    path_buf.to_string_lossy().to_string(),
                ));
                return Err(self.messages.clone());
            }
        };

        let mut raw_lines: Vec<(usize, String)> = Vec::new();
        let mut line_no = 1;

        for line in file.lines() {
            match line {
                Ok(l) => raw_lines.push((line_no, l)),
                Err(_) => self.messages.push(ParseMessage::new(
                    line_no,
                    ParseMessageCode::IOError,
                    "(unable to read line)".into(),
                )),
            }
            line_no += 1;
        }

        self.max_lines = raw_lines.len();

        // TODO: is there a better way to preallocate?
        self.records.reserve(raw_lines.len());
        self.namespace_index = HashMap::with_capacity(raw_lines.len());

        // pass 0: parse headers
        // pass 1: parse segments
        // pass 2: parse bridges (links/containments/jumps/gaps/edges/fragments)
        // pass 3: parse trails (paths/walks/groups)

        for pass in 0..4 {
            for &(idx, ref line) in &raw_lines {
                if matches!(line.as_bytes(), [] | [b'#', ..]) {
                    continue;
                }

                let tag = line.as_bytes()[0];
                match pass {
                    0 if tag != b'H' => continue,
                    1 if tag != b'S' => continue,
                    2 if (!(tag == b'L'
                        || tag == b'J'
                        || tag == b'C'
                        || tag == b'F'
                        || tag == b'E'
                        || tag == b'G')) =>
                    {
                        continue;
                    }
                    3 if (!(tag == b'P' || tag == b'W' || tag == b'O' || tag == b'U')) => {
                        continue;
                    }
                    _ => {}
                }

                // TODO: add current_line_no to GfaParser state so that we don't have to pass it around
                // or figure out a better way to handle error line numbers/context
                // my implementation is bad and ugly but it will take forever to refactor properly

                let (parsed_line, errs) =
                    GfaRecord::parse_line((self, line.as_str(), idx, options));

                self.push_record_and_update_index(parsed_line);

                self.messages.extend(errs);
            }
        }

        match self.header() {
            Some(header) => {
                if header.line_no != 1 {
                    self.messages.push(ParseMessage::new(
                        header.line_no,
                        ParseMessageCode::HeaderNotOnFirstLine,
                        header.raw.clone(),
                    ));
                }
            }
            None => {
                self.messages.push(ParseMessage::new(
                    0,
                    ParseMessageCode::MissingHeader,
                    path_buf.to_string_lossy().to_string(),
                ));
            }
        }

        self.add_info_errors();

        if self
            .messages
            .iter()
            .any(|e| e.severity() == ParseMessageSeverity::Fatal)
        {
            Err(self
                .messages
                .iter()
                .filter(|e| e.severity() == ParseMessageSeverity::Fatal)
                .cloned()
                .collect())
        } else {
            Ok(())
        }
    }

    /// Serialises the GFA records to a file.
    pub fn write_to_file(&self, path: &str, version: GFAVersion) -> Result<(), std::io::Error> {
        let path = PathBuf::from(path);
        let mut file = File::create(path)?;

        for pass in 0..4 {
            for record in &self.records {
                let record_pass = match record {
                    GfaRecord::Header(_) => 0,
                    GfaRecord::Segment(_) => 1,
                    GfaRecord::Link(_) => 2,
                    GfaRecord::Jump(_) => 2,
                    GfaRecord::Containment(_) => 2,
                    GfaRecord::Fragment(_) => 2,
                    GfaRecord::Edge(_) => 2,
                    GfaRecord::Gap(_) => 2,
                    GfaRecord::Path(_) => 3,
                    GfaRecord::Walk(_) => 3,
                    GfaRecord::OrderedGroup(_) => 3,
                    GfaRecord::UnorderedGroup(_) => 3,
                };
                if record_pass != pass {
                    continue;
                }
                let line = record.to_raw_line(version.clone(), self);
                if line.is_empty() {
                    continue;
                }
                writeln!(file, "{line}")?;
            }
        }
        Ok(())
    }

    /// Parses a raw GFA line and adds it to [`GfaParser::records`]. Returns the line number
    /// on Ok() or a `Vec` of errors if the line could not be parsed.
    ///
    /// Example:
    /// ```
    /// use parfait_gfa::gfa::{GfaParser, ParseOptions};
    ///
    /// let mut parser = GfaParser::new();
    /// let result = parser.add_line("S\ts1\tATCG", &ParseOptions::default());
    ///
    /// match result {
    ///     Ok(line_no) => println!("Added line at {}", line_no),
    ///     Err(errors) => println!("Failed to add line"),
    /// }
    /// ```
    pub fn add_line(
        &mut self,
        line: &str,
        options: &ParseOptions,
    ) -> Result<usize, Vec<ParseMessage>> {
        let line_no: usize = self.get_available_line_no();

        let (parsed_line, errs) = GfaRecord::parse_line((self, line, line_no, options));

        if parsed_line.is_none() {
            return Err(errs);
        } else {
            self.messages.extend(errs);
        }

        self.push_record_and_update_index(parsed_line);

        Ok(line_no)
    }

    /// Adds a clone of the GFA record to [`GfaParser::records`].
    /// Returns the line number on Ok() or a `Vec` of errors if the record could not be parsed.
    ///
    /// <div class="warning">
    /// To modify the record after it has been added,
    /// use any of the "find_mut_" methods with the line number returned from this method to
    /// obtain a mutable reference to the new record.
    /// </div>
    ///
    /// Example:
    /// ```
    /// use parfait_gfa::gfa::{GfaParser, ParseOptions};
    /// use parfait_gfa::line::segment::Segment;
    /// use parfait_gfa::line::record::GfaRecord;
    ///
    /// let mut parser = GfaParser::new();
    ///
    /// let new_segment = Segment {
    ///     name: "s1".into(),
    ///     sequence: "ATCG".into(),
    ///     length: Some(4),
    ///     ..Segment::default()
    /// };
    ///
    /// let result = parser.add_record(GfaRecord::Segment(new_segment), &ParseOptions::default());
    ///
    /// match result {
    ///     Ok(line_no) => println!("Added record at {}", line_no),
    ///     Err(errors) => println!("Failed to add record"),
    /// }
    /// ```
    pub fn add_record(
        &mut self,
        draft: GfaRecord,
        options: &ParseOptions,
    ) -> Result<usize, Vec<ParseMessage>> {
        let version = match self.version {
            GFAVersion::Unknown => GFAVersion::V2, // v2 preserves the most info
            _ => self.version.clone(),
        };

        let line = draft.to_raw_line(version, self);
        self.add_line(&line, options)
    }

    /// Returns the total length of all segments in the GFA file.
    ///
    /// Segment lengths are determined with the priority:
    /// 1. Length column (V2 only)
    /// 2. LN tag
    /// 3. Sequence length (if not `*`)
    ///
    /// All ghost segments have a length of 0.
    pub fn get_length(&self) -> u64 {
        self.segments().map(|s| s.get_length() as u64).sum()
    }

    /// Returns the first [`Header`] record, if any.
    pub fn header(&self) -> Option<&Header> {
        self.records.iter().find_map(GfaRecord::as_header)
    }

    /// Checks to see if a valid step exists between two segments.
    pub fn is_step_valid(
        &mut self,
        line_no: usize,
        from_segment_name: &str,
        to_segment_name: &str,
        from_orientation: bool,
        to_orientation: bool,
        allow_links: bool,
        allow_jumps: bool,
        allow_edges: bool,
        allow_gaps: bool,
        report_overlaps: bool,
    ) -> bool {
        let from_segment = &self.find_segment_with_name(from_segment_name);
        if from_segment.is_none() {
            return false;
        }

        let mut from_segment_outgoing_bridges = vec![];

        if allow_links {
            from_segment_outgoing_bridges.extend(match from_segment {
                Some(seg) => seg.outgoing_links.clone(),
                None => Vec::new(),
            });
        }

        if allow_jumps {
            from_segment_outgoing_bridges.extend(match from_segment {
                Some(seg) => seg.outgoing_jumps.clone(),
                None => Vec::new(),
            });
        }

        if allow_edges {
            from_segment_outgoing_bridges.extend(match from_segment {
                Some(seg) => seg.outgoing_edges.clone(),
                None => Vec::new(),
            });
        }

        if allow_gaps {
            from_segment_outgoing_bridges.extend(match from_segment {
                Some(seg) => seg.outgoing_gaps.clone(),
                None => Vec::new(),
            });
        }

        let to_segment = &self.find_segment_with_name(to_segment_name);
        if to_segment.is_none() {
            return false;
        }

        // Iterate over outgoing links of the from segment
        for bridge_idx in from_segment_outgoing_bridges {
            let bridge = &self.find_record_mut(bridge_idx);
            if bridge.is_none() {
                continue;
            }

            let bridge = bridge.as_ref().unwrap();

            let bridge_to_segment = match bridge {
                GfaRecord::Link(link) => link.to_segment.clone(),
                GfaRecord::Jump(jump) => jump.to_segment.clone(),
                GfaRecord::Edge(edge) => edge.to.reference.clone(),
                GfaRecord::Gap(gap) => gap.to.reference.clone(),
                _ => continue, // skip if not a bridge
            };

            // Check if the link goes to the correct segment
            if bridge_to_segment != to_segment_name {
                continue;
            }

            let bridge_from_orientation = match bridge {
                GfaRecord::Link(link) => link.from_orientation,
                GfaRecord::Jump(jump) => jump.from_orientation,
                GfaRecord::Edge(edge) => edge.from.direction,
                GfaRecord::Gap(gap) => gap.from.direction,
                _ => continue, // skip if not a bridge
            };

            let bridge_to_orientation = match bridge {
                GfaRecord::Link(link) => link.to_orientation,
                GfaRecord::Jump(jump) => jump.to_orientation,
                GfaRecord::Edge(edge) => edge.to.direction,
                GfaRecord::Gap(gap) => gap.to.direction,
                _ => continue, // skip if not a bridge
            };

            // Check if the orientations are correct
            if bridge_from_orientation != from_orientation
                || bridge_to_orientation != to_orientation
            {
                continue;
            }

            if report_overlaps
                && !(matches!(bridge, GfaRecord::Jump(_)) || matches!(bridge, GfaRecord::Gap(_)))
            {
                // if the link overlap is 0M, let it slide
                if (matches!(bridge, GfaRecord::Link(_))
                    && bridge.as_link().unwrap().overlap == "0M")
                {
                    return true;
                }

                let overlap = match bridge {
                    GfaRecord::Link(link) => link.overlap.clone(),
                    GfaRecord::Edge(edge) => {
                        format!("{} | {}", edge.from_interval, edge.to_interval)
                    }
                    _ => continue, // unreachable
                };

                let _ = &self.messages.push(ParseMessage::new(
                    line_no,
                    ParseMessageCode::WalkLinkHasOverlap,
                    format!(
                        "{}{} -> {}{} with overlap: {}",
                        from_segment_name,
                        if from_orientation { "+" } else { "-" },
                        to_segment_name,
                        if to_orientation { "+" } else { "-" },
                        overlap,
                    ),
                ));
            }

            return true;
        }

        false // no valid step found
    }

    /// Creates a new segment marked as a ghost.
    pub fn create_ghost_segment(&mut self, segment_name: String) -> &Segment {
        let mut new_seg = Segment::default();

        let line_no = self.get_available_line_no();
        let reference = self.ensure_name_unique(line_no, segment_name);

        new_seg.line_no = line_no;
        new_seg.name = reference.clone();
        new_seg.tags.add_flag("ghost");

        self.namespace_index
            .insert(reference.clone(), self.records.len());

        self.records_index
            .insert(new_seg.line_no, self.records.len());

        self.records.push(GfaRecord::Segment(new_seg));

        self.records
            .last()
            .and_then(|r| match r {
                GfaRecord::Segment(s) => Some(s),
                _ => None,
            })
            .unwrap()
    }

    /// Creates a new link marked as a ghost.
    pub fn create_ghost_link(
        &mut self,
        from_segment: String,
        from_orientation: bool,
        to_segment: String,
        to_orientation: bool,
        overlap: String,
    ) -> &Link {
        let mut new_link = Link::default();
        new_link.line_no = self.get_available_line_no();
        new_link.from_segment = from_segment;
        new_link.from_orientation = from_orientation;
        new_link.to_segment = to_segment;
        new_link.to_orientation = to_orientation;
        new_link.overlap = overlap;
        new_link.tags.add_flag("ghost");

        self.records_index
            .insert(new_link.line_no, self.records.len());

        self.records.push(GfaRecord::Link(new_link.clone()));

        self.records
            .last()
            .and_then(|r| match r {
                GfaRecord::Link(l) => Some(l),
                _ => None,
            })
            .unwrap()
    }

    /// Returns a unique name that is guaranteed not to collide with existing names.
    /// Calling this will add that name to the namespace.
    pub fn ensure_name_unique(&mut self, line_no: usize, name: String) -> String {
        if self.namespace.contains_key(&name) {
            self.messages.push(ParseMessage::new(
                line_no,
                ParseMessageCode::NamespaceCollision,
                name.to_owned(),
            ));

            // increment the occurrence of this name
            let occurrence = &self.namespace.get(&name).unwrap().clone();
            self.namespace.insert(name.clone(), occurrence + 1);

            // create a new name using the occurrence
            let new_name = format!("{}_{}", &name, occurrence + 1);
            self.namespace.insert(new_name.clone(), 0);

            return new_name;
        }

        self.namespace.insert(name.clone(), 0);
        name
    }

    /// Checks if a name is in the namespace.
    pub fn is_name_in_namespace(&self, name: &str) -> bool {
        self.namespace.contains_key(name)
    }

    /// Returns a count of dead-ends and all segments that are dead ends in the graph.
    /// NB: Isolated segments have two dead ends, hence the count may not always
    /// be equal to the length of the returned [`Vec<Segment>`]
    pub fn find_dead_ends(&mut self) -> (u32, Vec<&mut Segment>) {
        let mut dead_ends = 0;

        let segments: Vec<&mut Segment> = self
            .records
            .iter_mut()
            .filter_map(GfaRecord::as_mut_segment)
            .filter(|s| {
                let prev_dead_ends = dead_ends;

                if s.get_outgoing_bridges().is_empty() {
                    dead_ends += 1;
                }

                if s.get_incoming_bridges().is_empty() {
                    dead_ends += 1;
                }

                prev_dead_ends != dead_ends
            })
            .collect();

        (dead_ends, segments)
    }

    /// Finds all segments with no bridge connections
    pub fn find_isolated_segments(&mut self) -> Vec<&mut Segment> {
        self.records
            .iter_mut()
            .filter_map(GfaRecord::as_mut_segment)
            .filter(|s| s.get_outgoing_bridges().is_empty() && s.get_incoming_bridges().is_empty())
            .collect()
    }
}

/// Private helpers for GfaParser.
impl GfaParser {
    fn push_record_and_update_index(&mut self, parsed_line: Option<GfaRecord>) {
        if let Some(record) = parsed_line {
            // add to name index
            match &record {
                GfaRecord::Segment(s) => {
                    self.namespace_index
                        .insert(s.name.clone(), self.records.len());
                }
                GfaRecord::Path(p) => {
                    self.namespace_index
                        .insert(p.name.clone(), self.records.len());
                }
                GfaRecord::UnorderedGroup(ug) => {
                    self.namespace_index
                        .insert(ug.name.clone(), self.records.len());
                }
                GfaRecord::OrderedGroup(og) => {
                    self.namespace_index
                        .insert(og.name.clone(), self.records.len());
                }
                _ => {}
            }

            self.records_index
                .insert(record.line_no(), self.records.len());
            self.records.push(record);
        }
    }

    fn get_available_line_no(&mut self) -> usize {
        self.max_lines += 1;
        self.max_lines
    }

    fn add_info_errors(&mut self) {
        let isolated_segments = self
            .find_isolated_segments()
            .into_iter()
            .map(|s| (s.line_no, s.name.clone()))
            .collect::<Vec<_>>();

        for pair in isolated_segments {
            self.messages.push(ParseMessage::new(
                pair.0,
                ParseMessageCode::IsolatedSegment,
                pair.1,
            ));
        }

        let dead_end_segments = self
            .find_dead_ends()
            .1
            .into_iter()
            .map(|s| (s.line_no, s.name.clone()))
            .collect::<Vec<_>>();

        for pair in dead_end_segments {
            self.messages.push(ParseMessage::new(
                pair.0,
                ParseMessageCode::DeadEndTip,
                pair.1,
            ));
        }
    }
}

/// Iterators over GFA record types.
impl GfaParser {
    pub fn headers(&self) -> impl Iterator<Item = &Header> {
        self.records.iter().filter_map(GfaRecord::as_header)
    }

    pub fn segments(&self) -> impl Iterator<Item = &Segment> {
        self.records.iter().filter_map(GfaRecord::as_segment)
    }

    pub fn links(&self) -> impl Iterator<Item = &Link> {
        self.records.iter().filter_map(GfaRecord::as_link)
    }

    pub fn containments(&self) -> impl Iterator<Item = &Containment> {
        self.records.iter().filter_map(GfaRecord::as_containment)
    }

    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.records.iter().filter_map(GfaRecord::as_path)
    }

    pub fn walks(&self) -> impl Iterator<Item = &Walk> {
        self.records.iter().filter_map(GfaRecord::as_walk)
    }

    pub fn jumps(&self) -> impl Iterator<Item = &Jump> {
        self.records.iter().filter_map(GfaRecord::as_jump)
    }

    pub fn fragments(&self) -> impl Iterator<Item = &Fragment> {
        self.records.iter().filter_map(GfaRecord::as_fragment)
    }

    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.records.iter().filter_map(GfaRecord::as_edge)
    }

    pub fn gaps(&self) -> impl Iterator<Item = &Gap> {
        self.records.iter().filter_map(GfaRecord::as_gap)
    }

    pub fn unordered_groups(&self) -> impl Iterator<Item = &UnorderedGroup> {
        self.records
            .iter()
            .filter_map(GfaRecord::as_unordered_group)
    }

    pub fn ordered_groups(&self) -> impl Iterator<Item = &OrderedGroup> {
        self.records.iter().filter_map(GfaRecord::as_ordered_group)
    }

    pub fn headers_mut(&mut self) -> impl Iterator<Item = &mut Header> {
        self.records.iter_mut().filter_map(GfaRecord::as_mut_header)
    }

    pub fn segments_mut(&mut self) -> impl Iterator<Item = &mut Segment> {
        self.records
            .iter_mut()
            .filter_map(GfaRecord::as_mut_segment)
    }

    pub fn links_mut(&mut self) -> impl Iterator<Item = &mut Link> {
        self.records.iter_mut().filter_map(GfaRecord::as_mut_link)
    }

    pub fn containments_mut(&mut self) -> impl Iterator<Item = &mut Containment> {
        self.records
            .iter_mut()
            .filter_map(GfaRecord::as_mut_containment)
    }

    pub fn paths_mut(&mut self) -> impl Iterator<Item = &mut Path> {
        self.records.iter_mut().filter_map(GfaRecord::as_mut_path)
    }

    pub fn walks_mut(&mut self) -> impl Iterator<Item = &mut Walk> {
        self.records.iter_mut().filter_map(GfaRecord::as_mut_walk)
    }

    pub fn jumps_mut(&mut self) -> impl Iterator<Item = &mut Jump> {
        self.records.iter_mut().filter_map(GfaRecord::as_mut_jump)
    }

    pub fn fragments_mut(&mut self) -> impl Iterator<Item = &mut Fragment> {
        self.records
            .iter_mut()
            .filter_map(GfaRecord::as_mut_fragment)
    }

    pub fn edges_mut(&mut self) -> impl Iterator<Item = &mut Edge> {
        self.records.iter_mut().filter_map(GfaRecord::as_mut_edge)
    }

    pub fn gaps_mut(&mut self) -> impl Iterator<Item = &mut Gap> {
        self.records.iter_mut().filter_map(GfaRecord::as_mut_gap)
    }

    pub fn unordered_groups_mut(&mut self) -> impl Iterator<Item = &mut UnorderedGroup> {
        self.records
            .iter_mut()
            .filter_map(GfaRecord::as_mut_unordered_group)
    }

    pub fn ordered_groups_mut(&mut self) -> impl Iterator<Item = &mut OrderedGroup> {
        self.records
            .iter_mut()
            .filter_map(GfaRecord::as_mut_ordered_group)
    }
}

impl_enum_find_accessors! {
    for GfaParser;

    header  => Header,
    segment => Segment,
    link    => Link,
    path    => Path,
    containment => Containment,
    walk    => Walk,
    jump    => Jump,
    fragment => Fragment,
    edge    => Edge,
    gap     => Gap,
    unordered_group => UnorderedGroup,
    ordered_group => OrderedGroup,
}

impl GfaParser {
    pub fn find_record(&self, line_no: usize) -> Option<&GfaRecord> {
        let idx = self.records_index.get(&line_no)?;
        self.records.get(*idx)
    }

    pub fn find_record_mut(&mut self, line_no: usize) -> Option<&mut GfaRecord> {
        let idx = self.records_index.get(&line_no)?;
        self.records.get_mut(*idx)
    }

    pub fn find_segment_with_name(&mut self, name: &str) -> Option<&mut Segment> {
        let idx = self.namespace_index.get(name);
        self.records
            .get_mut(*idx?)
            .and_then(GfaRecord::as_mut_segment)
    }

    pub fn find_path_with_name(&mut self, name: &str) -> Option<&mut Path> {
        let idx = self.namespace_index.get(name);
        self.records.get_mut(*idx?).and_then(GfaRecord::as_mut_path)
    }

    pub fn find_unordered_group_with_name(&mut self, name: &str) -> Option<&mut UnorderedGroup> {
        let idx = self.namespace_index.get(name);
        self.records
            .get_mut(*idx?)
            .and_then(GfaRecord::as_mut_unordered_group)
    }

    pub fn find_ordered_group_with_name(&mut self, name: &str) -> Option<&mut OrderedGroup> {
        let idx = self.namespace_index.get(name);
        self.records
            .get_mut(*idx?)
            .and_then(GfaRecord::as_mut_ordered_group)
    }

    /// Get the associated line number of a record by its name
    pub fn find_line_no_with_name(&self, name: &str) -> Option<i32> {
        let idx = self.namespace_index.get(name)?;
        self.records.get(*idx).map(|r| r.line_no() as i32)
    }
}

/// Behaviour when a referenced segment does not exist in [GfaParser::records].
#[derive(Debug, Default, PartialEq, Eq, Clone, ValueEnum)]
pub enum MissingSegmentOptions {
    /// Create a ghost segment to satisfy the path step. A ghost segment is just a segment with a 0 length and a `ghost` tag.
    #[default]
    CreateGhost,
    /// Hard skip any bridge (link/jump/containment/edge) but only skip the path/walk/group step that references the missing segment.
    SoftSkip,
    /// Skip any line that references the missing segment.
    HardSkip,
    /// Continue parsing the line.
    Ignore,
}

impl std::fmt::Display for MissingSegmentOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MissingSegmentOptions::CreateGhost => write!(f, "create-ghost"),
            MissingSegmentOptions::SoftSkip => write!(f, "soft-skip"),
            MissingSegmentOptions::HardSkip => write!(f, "hard-skip"),
            MissingSegmentOptions::Ignore => write!(f, "ignore"),
        }
    }
}

/// Behaviour when a referenced bridge (including implicit references, such as a link between two path steps) does not exist in [GfaParser::records].
#[derive(Debug, Default, PartialEq, Eq, Clone, ValueEnum)]
pub enum MissingBridgeOptions {
    /// Create a ghost link to satisfy the path step.
    #[default]
    CreateGhostLink,
    /// Skip the path/walk/group entirely.
    HardSkip,
    /// Continue parsing the line.
    Ignore,
}

impl std::fmt::Display for MissingBridgeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MissingBridgeOptions::CreateGhostLink => write!(f, "create-ghost-link"),
            MissingBridgeOptions::HardSkip => write!(f, "hard-skip"),
            MissingBridgeOptions::Ignore => write!(f, "ignore"),
        }
    }
}

/// Options that can be passed to [GfaParser::parse]
/// to customise parsing behavior.
#[derive(Debug)]
pub struct ParseOptions {
    /// Skips checking if a sequence contains invalid characters, speeding up parsing of large GFA files.
    pub skip_invalid_sequence_test: bool,
    /// Store the raw lines of the GFA file in each record.
    /// This is only useful for debugging/error reporting.
    pub store_raw_lines: bool,
    /// Store segment sequences in memory. When [false], sequences will
    /// be replaced with `*` and an `LN` tag will be added if one does not exist.
    pub store_sequences: bool,
    /// When [true], if the path overlap column is omitted, the parser will use
    /// the overlaps from the links between each pair of segments in the path.
    /// This is only during validation and the derived overlaps are not exported.
    pub substitute_path_overlaps: bool,
    pub handle_missing_segment: MissingSegmentOptions,
    pub handle_missing_bridge: MissingBridgeOptions,
    /// Ignore errors produced when an implicit link can be used to satisfy
    /// a path step.
    ///
    /// Example: a path references a non-existent `-/-` link but a `+/+` link exists.
    pub allow_implicit_links: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            skip_invalid_sequence_test: false,
            store_raw_lines: false,
            store_sequences: true,
            substitute_path_overlaps: true,
            handle_missing_segment: MissingSegmentOptions::CreateGhost,
            handle_missing_bridge: MissingBridgeOptions::CreateGhostLink,
            allow_implicit_links: true,
        }
    }
}

/// GFA file format version.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum GFAVersion {
    V1,
    V1_1,
    V1_2,
    V2,
    #[default]
    Unknown,
}

impl From<String> for GFAVersion {
    fn from(val: String) -> Self {
        match val.as_str() {
            "1.0" => GFAVersion::V1,
            "1.1" => GFAVersion::V1_1,
            "1.2" => GFAVersion::V1_2,
            "2.0" => GFAVersion::V2,
            _ => GFAVersion::Unknown,
        }
    }
}

impl std::fmt::Display for GFAVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GFAVersion::V1 => write!(f, "1.0"),
            GFAVersion::V1_1 => write!(f, "1.1"),
            GFAVersion::V1_2 => write!(f, "1.2"),
            GFAVersion::V2 => write!(f, "2.0"),
            GFAVersion::Unknown => write!(f, "1.0"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gfa;

    #[test]
    fn no_parse_errors() {
        let mut newgfa = gfa::GfaParser::new();
        let parse_outcome = newgfa.parse(
            "test/gfa_working.gfa",
            &gfa::ParseOptions {
                skip_invalid_sequence_test: false,
                store_raw_lines: true,
                store_sequences: true,
                substitute_path_overlaps: true,
                handle_missing_segment: gfa::MissingSegmentOptions::CreateGhost,
                handle_missing_bridge: gfa::MissingBridgeOptions::CreateGhostLink,
                allow_implicit_links: true,
            },
        );

        for error in &newgfa.messages {
            error.print_formatted_error();
        }

        assert!(
            parse_outcome.is_ok(),
            "Failed to parse GFA file: {:?}",
            newgfa.messages
        );
        assert!(
            newgfa.messages.is_empty(),
            "There should be no parse errors, but found: {:?}",
            newgfa.messages
        );
    }

    #[test]
    fn roundtrip() {
        let mut newgfa = gfa::GfaParser::new();
        let parse_outcome = newgfa.parse(
            "test/gfa_working.gfa",
            &gfa::ParseOptions {
                skip_invalid_sequence_test: false,
                store_raw_lines: false,
                store_sequences: true,
                substitute_path_overlaps: true,
                handle_missing_segment: gfa::MissingSegmentOptions::Ignore,
                handle_missing_bridge: gfa::MissingBridgeOptions::Ignore,
                allow_implicit_links: true,
            },
        );

        for error in &newgfa.messages {
            error.print_formatted_error();
        }

        newgfa
            .write_to_file("test/gfa_working_roundtrip.gfa", gfa::GFAVersion::V1_2)
            .expect("Failed to save GFA file");

        let mut newgfa2 = gfa::GfaParser::new();
        let parse_outcome2 = newgfa2.parse(
            "test/gfa_working_roundtrip.gfa",
            &gfa::ParseOptions {
                skip_invalid_sequence_test: false,
                store_raw_lines: false,
                store_sequences: true,
                substitute_path_overlaps: true,
                handle_missing_segment: gfa::MissingSegmentOptions::Ignore,
                handle_missing_bridge: gfa::MissingBridgeOptions::Ignore,
                allow_implicit_links: true,
            },
        );

        for error in &newgfa2.messages {
            error.print_formatted_error();
        }

        assert!(
            parse_outcome2.is_ok(),
            "Failed to parse GFA file: {:?}",
            newgfa2.messages
        );

        assert_eq!(
            newgfa.records.len(),
            newgfa2.records.len(),
            "Record count mismatch after roundtrip"
        );
        assert_eq!(
            newgfa.messages.len(),
            newgfa2.messages.len(),
            "Error count mismatch after roundtrip"
        );

        assert!(
            parse_outcome.is_ok(),
            "Failed to parse GFA file: {:?}",
            newgfa.messages
        );
    }
}
