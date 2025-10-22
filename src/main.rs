use clap::Parser;
use std::io::{self};
use owo_colors::OwoColorize;
use parfait_gfa::{errors::ParseMessageSeverity, gfa::{GfaParser, MissingBridgeOptions, MissingSegmentOptions, ParseOptions}};

/// A simple GFA parser application
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// path to the GFA file
    #[arg(required = true, index=1)]
    path: String,

    /// when the path overlaps field is omitted, don't attempt to derive it from the link overlap
    #[arg(short, long, default_value_t = false)]
    never_derive_path_overlaps: bool,

    /// how missing segments should be handled
    ///     create-ghost: creates a ghost segment
    ///     soft-skip: hard skip any bridge but only skip the path/walk step that references the missing segment
    ///     hard-skip: skip any line that references the missing segment
    ///     ignore: do nothing but log the error
    #[arg(long, default_value_t = MissingSegmentOptions::SoftSkip, verbatim_doc_comment)]
    missing_segments: MissingSegmentOptions,

    /// how missing bridges should be handled
    ///     create-ghost-link: always creates a link
    ///     hard-skip: skips the path/walk entirely
    ///     ignore: do nothing but log the error
    #[arg(long, default_value_t = MissingBridgeOptions::HardSkip, verbatim_doc_comment)]
    missing_bridges: MissingBridgeOptions,

    
    /// filter errors by severity (i: info, w: warn, s: severe, e: error, f: fatal)
    /// 
    /// example: `-f iw` will stop info and warnings from being printed
    #[arg(short, long, default_value_t = String::from(""))]
    filter_severity: String,

    /// ignore errors produced when an implicit link already exists
    /// example: a path references a non-existant -/- link but a +/+ link exists
    #[arg(long, default_value_t = true)]
    allow_implicit_links: bool,

    /// don't print any messages, only the final summary
    #[arg(short, long, default_value_t = false)]
    quiet: bool,
}

fn print_record_count<T>(name: &str, records: impl Iterator<Item = T>) {
    let count = records.count();
    if count > 0 {
        println!("{name}: {count}");
    }
}

use si_scale::scale_fn;
scale_fn!(base_pairs,
    base: B1000,
    constraint: UnitAndAbove,
    mantissa_fmt: "{:.2}",
    groupings: '_',
    unit: "bp",
    doc: "si base pairs"
);

fn main() -> io::Result<()> {
    let args = Args::parse();

    let path = args.path;
    let mut gfa = GfaParser::new();

    let options = ParseOptions {
        skip_invalid_sequence_test: true,
        store_raw_lines: false,
        store_sequences: false,
        substitute_path_overlaps: !args.never_derive_path_overlaps,
        handle_missing_segment: args.missing_segments,
        handle_missing_bridge: args.missing_bridges,
        allow_implicit_links: args.allow_implicit_links,
    };

    let result = gfa.parse(path, &options);
    
    if !args.quiet {
        for error in &gfa.messages {
            if args.filter_severity.contains(error.severity().to_char()) {
                continue;
            }
            error.print_formatted_error();
        }
    }

    match result {
        Ok(_) => {
            println!(
                "{}",
                "[*] [parfait-gfa] Successfully parsed GFA file".to_string()
                    .on_green()
                    .bold()
            );
        }

        Err(_) => {
            println!(
                "{}",
                "[!] [parfait-gfa] Failed to parse GFA file".to_string()
                    .on_red()
                    .bold()
            );
        }
    }

    let err_counts = gfa.messages.iter().fold(
        (0, 0, 0, 0, 0),
        |(fatal, error, severe, warning, info), e| match e.severity() {
            ParseMessageSeverity::Fatal => (fatal + 1, error, severe, warning, info),
            ParseMessageSeverity::Error => (fatal, error + 1, severe, warning, info),
            ParseMessageSeverity::Severe => (fatal, error, severe + 1, warning, info),
            ParseMessageSeverity::Warn => (fatal, error, severe, warning + 1, info),
            ParseMessageSeverity::Info => (fatal, error, severe, warning, info + 1),
        },
    );

    println!("{}", format!("[X] fatal: {}", err_counts.0).magenta());
    println!("{}", format!("[!] error: {}", err_counts.1).bright_red());
    println!("{}", format!("[#] severe: {}", err_counts.2).red());
    println!("{}", format!("[?] warning: {}", err_counts.3).yellow());
    println!("{}", format!("[*] info: {}", err_counts.4).blue());

    println!();

    print_record_count("headers", gfa.headers());
    print_record_count("segments", gfa.segments());
    print_record_count("links", gfa.links());
    print_record_count("jumps", gfa.jumps());
    print_record_count("containments", gfa.containments());
    print_record_count("paths", gfa.paths());
    print_record_count("walks", gfa.walks());
    print_record_count("edges", gfa.edges());
    print_record_count("fragments", gfa.fragments());
    print_record_count("gaps", gfa.gaps());

    println!();    

    println!("length: {} bp ({})", gfa.get_length(), base_pairs(gfa.get_length() as f64));

    Ok(())
}
