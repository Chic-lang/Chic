use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const UCD_BASE: &str = "https://www.unicode.org/Public/17.0.0/ucd/";
const AUX_BASE: &str = "https://www.unicode.org/Public/17.0.0/ucd/auxiliary/";
const EMOJI_BASE: &str = "https://www.unicode.org/Public/17.0.0/ucd/emoji/";

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from("generated/unicode17");
    fs::create_dir_all(&out_dir)?;
    let cache_dir = out_dir.join("ucd_cache");
    fs::create_dir_all(&cache_dir)?;

    let unicode_data = fetch_ucd(
        &cache_dir,
        "UnicodeData.txt",
        format!("{UCD_BASE}UnicodeData.txt"),
    )?;
    let derived_core = fetch_ucd(
        &cache_dir,
        "DerivedCoreProperties.txt",
        format!("{UCD_BASE}DerivedCoreProperties.txt"),
    )?;
    let derived_norm = fetch_ucd(
        &cache_dir,
        "DerivedNormalizationProps.txt",
        format!("{UCD_BASE}DerivedNormalizationProps.txt"),
    )?;
    let comp_exclusions = fetch_ucd(
        &cache_dir,
        "CompositionExclusions.txt",
        format!("{UCD_BASE}CompositionExclusions.txt"),
    )?;
    let grapheme_break = fetch_ucd(
        &cache_dir,
        "GraphemeBreakProperty.txt",
        format!("{AUX_BASE}GraphemeBreakProperty.txt"),
    )?;
    let emoji_data = fetch_ucd(
        &cache_dir,
        "emoji-data.txt",
        format!("{EMOJI_BASE}emoji-data.txt"),
    )?;

    let id_start = parse_property_ranges(&derived_core, "ID_Start");
    let id_continue = parse_property_ranges(&derived_core, "ID_Continue");
    let pattern_ws = parse_property_ranges(&derived_core, "Pattern_White_Space");
    let pattern_syntax = parse_property_ranges(&derived_core, "Pattern_Syntax");

    let (ccc_map, decomp_map) = parse_unicode_data(&unicode_data);
    let mut exclusions = parse_range_set(&comp_exclusions);
    for (start, end) in parse_property_ranges(&derived_norm, "Full_Composition_Exclusion") {
        for scalar in start..=end {
            exclusions.insert(scalar);
        }
    }
    let compositions = build_compositions(&decomp_map, &exclusions);

    let grapheme_props = parse_grapheme_break(&grapheme_break);
    let ext_pict = parse_property_ranges(&emoji_data, "Extended_Pictographic");

    write_ident_tables(&out_dir, id_start, id_continue, pattern_ws, pattern_syntax)?;
    write_normalization_tables(&out_dir, &ccc_map, &decomp_map, &compositions)?;
    write_grapheme_tables(&out_dir, &grapheme_props, &ext_pict)?;

    println!("generated Unicode 17 tables into {}", out_dir.display());
    Ok(())
}

fn fetch_ucd(
    cache_dir: &Path,
    name: &str,
    url: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let path = cache_dir.join(name);
    if let Ok(contents) = fs::read_to_string(&path) {
        return Ok(contents);
    }
    println!("downloading {url}");
    let response = ureq::get(&url).call()?;
    let text = response.into_string()?;
    fs::write(&path, text.as_bytes())?;
    Ok(text)
}

fn parse_property_ranges(text: &str, property: &str) -> Vec<(u32, u32)> {
    let mut ranges = Vec::new();
    for line in text.lines() {
        let Some((range_text, prop_text)) = line.split_once(';') else {
            continue;
        };
        if !prop_text.trim().starts_with(property) {
            continue;
        }
        if let Some((start, end)) = parse_range(range_text) {
            ranges.push((start, end));
        }
    }
    merge_ranges(ranges)
}

fn parse_range_set(text: &str) -> HashSet<u32> {
    let mut set = HashSet::new();
    for line in text.lines() {
        let Some((range_text, _)) = line.split_once(';') else {
            continue;
        };
        if let Some((start, end)) = parse_range(range_text) {
            for scalar in start..=end {
                set.insert(scalar);
            }
        }
    }
    set
}

fn parse_range(text: &str) -> Option<(u32, u32)> {
    let mut trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(idx) = trimmed.find('#') {
        trimmed.truncate(idx);
    }
    let cleaned = trimmed.trim();
    if cleaned.is_empty() {
        return None;
    }
    if let Some((start, end)) = cleaned.split_once("..") {
        let start = u32::from_str_radix(start.trim(), 16).ok()?;
        let end = u32::from_str_radix(end.trim(), 16).ok()?;
        Some((start, end))
    } else {
        let value = u32::from_str_radix(cleaned.trim(), 16).ok()?;
        Some((value, value))
    }
}

fn parse_unicode_data(text: &str) -> (HashMap<u32, u8>, HashMap<u32, Vec<u32>>) {
    let mut ccc_map = HashMap::new();
    let mut decomp_map = HashMap::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(';').collect();
        if fields.len() < 6 {
            continue;
        }
        let code = match u32::from_str_radix(fields[0], 16) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if let Ok(class) = fields[3].parse::<u8>() {
            if class != 0 {
                ccc_map.insert(code, class);
            }
        }
        let decomp_field = fields[5].trim();
        if decomp_field.is_empty() {
            continue;
        }
        if decomp_field.starts_with('<') {
            // Compatibility or algorithmic; canonical decomposition entries never include tags.
            continue;
        }
        let scalars: Vec<u32> = decomp_field
            .split_whitespace()
            .filter_map(|value| u32::from_str_radix(value, 16).ok())
            .collect();
        if scalars.len() > 1 {
            decomp_map.insert(code, scalars);
        } else if scalars.len() == 1 {
            decomp_map.insert(code, scalars);
        }
    }

    (ccc_map, decomp_map)
}

fn build_compositions(
    decomp_map: &HashMap<u32, Vec<u32>>,
    exclusions: &HashSet<u32>,
) -> Vec<(u32, u32, u32)> {
    let mut compositions = Vec::new();
    for (&scalar, mapping) in decomp_map {
        if mapping.len() != 2 {
            continue;
        }
        if exclusions.contains(&scalar) {
            continue;
        }
        if is_hangul_syllable(scalar) {
            continue;
        }
        compositions.push((mapping[0], mapping[1], scalar));
    }
    compositions.sort_unstable();
    compositions.dedup();
    compositions
}

fn parse_grapheme_break(text: &str) -> HashMap<String, Vec<(u32, u32)>> {
    let mut properties: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
    for line in text.lines() {
        let Some((range_text, prop_text)) = line.split_once(';') else {
            continue;
        };
        let property = prop_text.split('#').next().unwrap_or("").trim();
        if property.is_empty() {
            continue;
        }
        if let Some((start, end)) = parse_range(range_text) {
            properties
                .entry(property.to_string())
                .or_default()
                .push((start, end));
        }
    }
    properties
        .into_iter()
        .map(|(prop, ranges)| (prop, merge_ranges(ranges)))
        .collect()
}

fn write_ident_tables(
    out_dir: &Path,
    mut id_start: Vec<(u32, u32)>,
    mut id_continue: Vec<(u32, u32)>,
    mut pattern_ws: Vec<(u32, u32)>,
    mut pattern_syntax: Vec<(u32, u32)>,
) -> Result<(), Box<dyn std::error::Error>> {
    id_start = merge_ranges(id_start);
    id_continue = merge_ranges(id_continue);
    pattern_ws = merge_ranges(pattern_ws);
    pattern_syntax = merge_ranges(pattern_syntax);

    let mut file = fs::File::create(out_dir.join("ident.rs"))?;
    writeln!(
        file,
        "// @generated by `cargo xtask unicode17`\n// Unicode identifier property ranges (Unicode 17.0.0)."
    )?;
    write_ranges(&mut file, "ID_START_RANGES", id_start)?;
    write_ranges(&mut file, "ID_CONTINUE_RANGES", id_continue)?;
    write_ranges(&mut file, "PATTERN_WHITE_SPACE_RANGES", pattern_ws)?;
    write_ranges(&mut file, "PATTERN_SYNTAX_RANGES", pattern_syntax)?;
    Ok(())
}

fn write_normalization_tables(
    out_dir: &Path,
    ccc_map: &HashMap<u32, u8>,
    decomp_map: &HashMap<u32, Vec<u32>>,
    compositions: &[(u32, u32, u32)],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = fs::File::create(out_dir.join("normalization.rs"))?;
    writeln!(
        file,
        "// @generated by `cargo xtask unicode17`\n// Canonical combining classes and decomposition/composition tables for Unicode 17.0.0."
    )?;

    // Canonical combining classes (sparse)
    let mut ccc_entries: Vec<(u32, u8)> = ccc_map.iter().map(|(k, v)| (*k, *v)).collect();
    ccc_entries.sort_unstable();
    writeln!(
        file,
        "pub const CANONICAL_COMBINING_CLASSES: &[(u32, u8)] = &["
    )?;
    for (scalar, class) in ccc_entries {
        writeln!(file, "    (0x{scalar:04X}, {class}),")?;
    }
    writeln!(file, "];\n")?;

    // Canonical decompositions (including length-one entries for lookup parity).
    let mut decomp_entries: Vec<(u32, Vec<u32>)> =
        decomp_map.iter().map(|(k, v)| (*k, v.clone())).collect();
    decomp_entries.sort_by_key(|(scalar, _)| *scalar);
    writeln!(
        file,
        "pub const CANONICAL_DECOMPOSITIONS: &[(u32, &[u32])] = &["
    )?;
    for (scalar, mapping) in decomp_entries {
        write!(file, "    (0x{scalar:04X}, &[")?;
        for (idx, part) in mapping.iter().enumerate() {
            if idx > 0 {
                write!(file, ", ")?;
            }
            write!(file, "0x{part:04X}")?;
        }
        writeln!(file, "]),")?;
    }
    writeln!(file, "];\n")?;

    let mut comp_entries = compositions.to_vec();
    comp_entries.sort_unstable();
    writeln!(
        file,
        "pub const CANONICAL_COMPOSITIONS: &[(u32, u32, u32)] = &["
    )?;
    for (starter, combining, composite) in comp_entries {
        writeln!(
            file,
            "    (0x{starter:04X}, 0x{combining:04X}, 0x{composite:04X}),"
        )?;
    }
    writeln!(file, "];")?;
    Ok(())
}

fn write_grapheme_tables(
    out_dir: &Path,
    properties: &HashMap<String, Vec<(u32, u32)>>,
    ext_pict: &[(u32, u32)],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = fs::File::create(out_dir.join("grapheme.rs"))?;
    writeln!(
        file,
        "// @generated by `cargo xtask unicode17`\n// Grapheme break property ranges (Unicode 17.0.0)."
    )?;

    let mut merged: Vec<(u32, u32, String)> = Vec::new();
    for (name, ranges) in properties {
        let prop_ident = grapheme_prop_ident(name);
        for (start, end) in ranges {
            merged.push((*start, *end, prop_ident.clone()));
        }
    }
    merged.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    writeln!(
        file,
        "pub const GRAPHEME_BREAK_RANGES: &[GraphemeBreakRange] = &["
    )?;
    for (start, end, prop_ident) in merged {
        writeln!(
            file,
            "    GraphemeBreakRange {{ start: 0x{start:04X}, end: 0x{end:04X}, property: GraphemeBreakProperty::{prop_ident} }},"
        )?;
    }
    writeln!(file, "];\n")?;

    let mut ext_pict_ranges = merge_ranges(ext_pict.to_vec());
    writeln!(
        file,
        "pub const EXTENDED_PICTOGRAPHIC_RANGES: &[Range] = &["
    )?;
    ext_pict_ranges.sort_unstable();
    for (start, end) in ext_pict_ranges {
        writeln!(
            file,
            "    Range {{ start: 0x{start:04X}, end: 0x{end:04X} }},"
        )?;
    }
    writeln!(file, "];")?;
    Ok(())
}

fn grapheme_prop_ident(name: &str) -> String {
    name.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<String>()
}

fn write_ranges(
    file: &mut fs::File,
    name: &str,
    mut ranges: Vec<(u32, u32)>,
) -> Result<(), Box<dyn std::error::Error>> {
    ranges.sort_unstable();
    writeln!(file, "pub const {name}: &[Range] = &[")?;
    for (start, end) in ranges {
        writeln!(
            file,
            "    Range {{ start: 0x{start:04X}, end: 0x{end:04X} }},"
        )?;
    }
    writeln!(file, "];\n")?;
    Ok(())
}

fn merge_ranges(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    if ranges.is_empty() {
        return ranges;
    }
    ranges.sort_unstable();
    let mut merged = Vec::with_capacity(ranges.len());
    let mut current = ranges[0];
    for (start, end) in ranges.into_iter().skip(1) {
        if start <= current.1 + 1 {
            current.1 = current.1.max(end);
        } else {
            merged.push(current);
            current = (start, end);
        }
    }
    merged.push(current);
    merged
}

fn is_hangul_syllable(value: u32) -> bool {
    (0xAC00..=0xD7A3).contains(&value)
}
