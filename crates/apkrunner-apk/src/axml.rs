//! Minimal, dependency-free parser for Android's binary XML (AXML).
//!
//! `AndroidManifest.xml` inside an APK is not text: it is a chunked binary
//! format (`ResXMLTree`). This module decodes it into a small DOM that is
//! enough to read packages, SDK levels, activities and intent filters.

use crate::error::{Error, Result};

pub(crate) const RES_XML_TYPE: u16 = 0x0003;
pub(crate) const RES_STRING_POOL_TYPE: u16 = 0x0001;
pub(crate) const RES_XML_RESOURCE_MAP_TYPE: u16 = 0x0180;
pub(crate) const RES_XML_START_ELEMENT_TYPE: u16 = 0x0102;
pub(crate) const RES_XML_END_ELEMENT_TYPE: u16 = 0x0103;

pub(crate) const NO_ENTRY: u32 = 0xFFFF_FFFF;
const MAX_DEPTH: usize = 256;

/// A typed attribute value, as stored in the XML chunk.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Empty,
    Str(String),
    Int(i64),
    Bool(bool),
    /// A reference into `resources.arsc` (not resolved yet).
    Reference(u32),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Value::Int(i) if *i >= 0 && *i <= u32::MAX as i64 => Some(*i as u32),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            Value::Int(i) => Some(*i != 0),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub namespace: Option<String>,
    pub name: String,
    pub value: Value,
    /// Raw string, when Android stored one alongside the typed value.
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Element>,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.name == name)
    }

    pub fn attr_str(&self, name: &str) -> Option<&str> {
        self.attr(name).and_then(|a| a.value.as_str())
    }

    pub fn attr_u32(&self, name: &str) -> Option<u32> {
        self.attr(name).and_then(|a| a.value.as_u32())
    }

    pub fn attr_bool(&self, name: &str) -> Option<bool> {
        self.attr(name).and_then(|a| a.value.as_bool())
    }

    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> {
        self.children.iter().filter(move |c| c.name == name)
    }
}

/// Parse a binary `AndroidManifest.xml` into its root element.
pub fn parse(data: &[u8]) -> Result<Element> {
    if data.len() < 8 {
        return Err(Error::Manifest("file is too short".into()));
    }
    let kind = read_u16(data, 0)?;
    if kind != RES_XML_TYPE {
        return Err(Error::Manifest(format!(
            "unexpected chunk type {kind:#06x} (expected XML)"
        )));
    }

    let mut window_start = read_u16(data, 2)? as usize;
    if window_start < 8 {
        window_start = 8;
    }

    let mut strings: Vec<String> = Vec::new();
    let mut resource_ids: Vec<u32> = Vec::new();
    let mut stack: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;

    let mut pos = window_start;
    while pos + 8 <= data.len() {
        let chunk_type = read_u16(data, pos)?;
        let header_size = read_u16(data, pos + 2)? as usize;
        let chunk_size = read_u32(data, pos + 4)? as usize;

        if chunk_size < 8 || header_size < 8 || header_size > chunk_size {
            return Err(Error::Manifest(format!(
                "bad chunk at {pos:#x}: size={chunk_size} header={header_size}"
            )));
        }
        let end = pos
            .checked_add(chunk_size)
            .filter(|e| *e <= data.len())
            .ok_or_else(|| Error::Manifest("chunk overflows file".into()))?;

        match chunk_type {
            RES_STRING_POOL_TYPE => {
                strings = parse_string_pool(data, pos, header_size, chunk_size)?;
            }
            RES_XML_RESOURCE_MAP_TYPE => {
                resource_ids.clear();
                let mut p = pos + header_size;
                while p + 4 <= end {
                    resource_ids.push(read_u32(data, p)?);
                    p += 4;
                }
            }
            RES_XML_START_ELEMENT_TYPE => {
                if stack.len() >= MAX_DEPTH {
                    return Err(Error::Manifest("XML nesting too deep".into()));
                }
                let element = parse_start_element(
                    data,
                    pos,
                    header_size,
                    chunk_size,
                    &strings,
                    &resource_ids,
                )?;
                stack.push(element);
            }
            RES_XML_END_ELEMENT_TYPE => {
                if let Some(element) = stack.pop() {
                    match stack.last_mut() {
                        Some(parent) => parent.children.push(element),
                        None => root = Some(element),
                    }
                }
            }
            _ => {} // namespaces and CDATA are not needed for the manifest
        }

        pos = end;
    }

    root.ok_or_else(|| Error::Manifest("no root element".into()))
}

fn parse_string_pool(
    data: &[u8],
    offset: usize,
    header_size: usize,
    chunk_size: usize,
) -> Result<Vec<String>> {
    let end = offset + chunk_size;
    let count = read_u32(data, offset + 8)? as usize;
    let flags = read_u32(data, offset + 16)?;
    let strings_start = read_u32(data, offset + 20)? as usize;
    let is_utf8 = flags & 0x100 != 0;

    let offsets_start = offset + header_size;
    let base = offset
        .checked_add(strings_start)
        .filter(|b| *b <= end)
        .ok_or_else(|| Error::Manifest("string pool offset out of bounds".into()))?;

    // Reject absurd counts before allocating.
    if count > 1_000_000 || offsets_start + count * 4 > end {
        return Err(Error::Manifest("bad string pool table".into()));
    }

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let rel = read_u32(data, offsets_start + i * 4)? as usize;
        let p = base + rel;
        if p >= end {
            return Err(Error::Manifest("string offset out of bounds".into()));
        }
        out.push(if is_utf8 {
            read_utf8_string(data, p, end)?
        } else {
            read_utf16_string(data, p, end)?
        });
    }
    Ok(out)
}

fn read_utf8_string(data: &[u8], mut p: usize, end: usize) -> Result<String> {
    let (len, consumed) = read_len8(data, p, end)?;
    p += consumed;
    let bytes = data
        .get(p..p + len)
        .filter(|_| p + len <= end)
        .ok_or_else(|| Error::Manifest("truncated UTF-8 string".into()))?;
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

fn read_len8(data: &[u8], p: usize, end: usize) -> Result<(usize, usize)> {
    let b0 = *data
        .get(p)
        .filter(|_| p < end)
        .ok_or_else(|| Error::Manifest("truncated string length".into()))?;
    if b0 & 0x80 != 0 {
        let b1 = *data
            .get(p + 1)
            .filter(|_| p + 1 < end)
            .ok_or_else(|| Error::Manifest("truncated string length".into()))?;
        Ok(((((b0 & 0x7f) as usize) << 8) | b1 as usize, 2))
    } else {
        Ok((b0 as usize, 1))
    }
}

fn read_utf16_string(data: &[u8], mut p: usize, end: usize) -> Result<String> {
    let first = read_u16(data, p)?;
    let (len, consumed) = if first & 0x8000 != 0 {
        let second = read_u16(data, p + 2)?;
        ((((first & 0x7fff) as usize) << 16) | second as usize, 4)
    } else {
        (first as usize, 2)
    };
    p += consumed;
    if len > 1_000_000 || p + len * 2 > end {
        return Err(Error::Manifest("truncated UTF-16 string".into()));
    }
    let units: Vec<u16> = (0..len)
        .map(|i| read_u16(data, p + i * 2))
        .collect::<Result<_>>()?;
    Ok(String::from_utf16_lossy(&units))
}

fn parse_start_element(
    data: &[u8],
    offset: usize,
    header_size: usize,
    chunk_size: usize,
    strings: &[String],
    resource_ids: &[u32],
) -> Result<Element> {
    // In real APKs `header_size` is only 16 (ResXMLTree_node): the
    // ResXMLTree_attrExt block is part of the chunk body and holds its own
    // offset to the attribute table.
    if header_size < 16 {
        return Err(Error::Manifest("start element header too small".into()));
    }
    let chunk_end = offset
        .checked_add(chunk_size)
        .ok_or_else(|| Error::Manifest("element chunk overflow".into()))?;

    let name_index = read_u32(data, offset + 20)?;
    let name = string_at(strings, name_index)
        .ok_or_else(|| Error::Manifest("element name out of range".into()))?
        .to_string();

    let attr_start = read_u16(data, offset + 24)? as usize;
    let attr_size = read_u16(data, offset + 26)? as usize;
    let attr_count = read_u16(data, offset + 28)? as usize;

    if attr_size < 20 || attr_count > 4096 {
        return Err(Error::Manifest("bad attribute table".into()));
    }

    let attrs_at = offset
        .checked_add(16 + attr_start)
        .ok_or_else(|| Error::Manifest("attribute offset overflow".into()))?;
    let table_end = attrs_at
        .checked_add(attr_count * attr_size)
        .ok_or_else(|| Error::Manifest("attribute table overflow".into()))?;
    if table_end > chunk_end {
        return Err(Error::Manifest("attribute table overflows chunk".into()));
    }

    let mut attributes = Vec::with_capacity(attr_count);
    for i in 0..attr_count {
        let a = attrs_at + i * attr_size;
        let ns_index = read_u32(data, a)?;
        let attr_name_index = read_u32(data, a + 4)?;
        let raw_index = read_u32(data, a + 8)?;
        let data_type = *data
            .get(a + 15)
            .ok_or_else(|| Error::Manifest("truncated attribute".into()))?;
        let value_data = read_u32(data, a + 16)?;

        let mut attr_name = string_at(strings, attr_name_index)
            .unwrap_or("")
            .to_string();
        if attr_name.is_empty() {
            // Some obfuscated manifests blank the names and rely on the
            // resource map. Fall back to well-known android: IDs.
            if let Some(id) = resource_ids.get(attr_name_index as usize) {
                attr_name = known_attr_name(*id);
            }
        }
        if attr_name.is_empty() {
            continue;
        }

        let namespace = string_at(strings, ns_index)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let raw = if raw_index == NO_ENTRY {
            None
        } else {
            string_at(strings, raw_index).map(str::to_string)
        };

        attributes.push(Attribute {
            namespace,
            name: attr_name,
            value: typed_value(data_type, value_data, strings)?,
            raw,
        });
    }

    Ok(Element {
        name,
        attributes,
        children: Vec::new(),
    })
}

fn typed_value(data_type: u8, data: u32, strings: &[String]) -> Result<Value> {
    Ok(match data_type {
        0x00 | 0x02 => Value::Empty,
        0x01 => Value::Reference(data),
        0x03 => match string_at(strings, data) {
            Some(s) => Value::Str(s.to_string()),
            None => Value::Empty,
        },
        0x12 => Value::Bool(data != 0),
        // int dec/hex, colors, dimensions and fractions: keep the integer
        // payload, which is what we need for SDK levels and booleans.
        _ => Value::Int(data as i32 as i64),
    })
}

fn string_at(strings: &[String], index: u32) -> Option<&str> {
    if index == NO_ENTRY {
        return None;
    }
    strings.get(index as usize).map(String::as_str)
}

fn known_attr_name(id: u32) -> String {
    // Well-known android:* attribute resource IDs. Only the ones the
    // inspector needs; unknown IDs are ignored by the caller.
    let name = match id {
        0x0101_0001 => "label",
        0x0101_0002 => "icon",
        0x0101_0003 => "name",
        0x0101_0004 => "permission",
        0x0101_000e => "enabled",
        0x0101_000f => "debuggable",
        0x0101_0010 => "exported",
        0x0101_0011 => "process",
        0x0101_001b => "grantUriPermissions",
        0x0101_001d => "launchMode",
        0x0101_001e => "screenOrientation",
        0x0101_0021 => "targetPackage",
        0x0101_0024 => "value",
        0x0101_0025 => "resource",
        0x0101_020c => "minSdkVersion",
        0x0101_021b => "versionCode",
        0x0101_021c => "versionName",
        0x0101_0270 => "targetSdkVersion",
        0x0101_0271 => "maxSdkVersion",
        0x0101_04ea => "extractNativeLibs",
        0x0101_052c => "roundIcon",
        _ => return format!("@{id:#010x}"),
    };
    name.to_string()
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| Error::Manifest("unexpected end of file".into()))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| Error::Manifest("unexpected end of file".into()))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{el, encode, with, AttrValue};

    #[test]
    fn parses_manifest_basics() {
        let manifest = with(
            el(
                "manifest",
                vec![
                    ("package", AttrValue::Str("com.example.hello")),
                    ("versionCode", AttrValue::Int(7)),
                    ("versionName", AttrValue::Str("1.2")),
                ],
            ),
            vec![
                el(
                    "uses-sdk",
                    vec![
                        ("minSdkVersion", AttrValue::Int(21)),
                        ("targetSdkVersion", AttrValue::Int(34)),
                    ],
                ),
                with(
                    el("application", vec![("label", AttrValue::Str("Hello"))]),
                    vec![with(
                        el(
                            "activity",
                            vec![
                                ("name", AttrValue::Str(".MainActivity")),
                                ("exported", AttrValue::Bool(true)),
                            ],
                        ),
                        vec![with(
                            el("intent-filter", vec![]),
                            vec![
                                el(
                                    "action",
                                    vec![("name", AttrValue::Str("android.intent.action.MAIN"))],
                                ),
                                el(
                                    "category",
                                    vec![(
                                        "name",
                                        AttrValue::Str("android.intent.category.LAUNCHER"),
                                    )],
                                ),
                            ],
                        )],
                    )],
                ),
            ],
        );

        let root = parse(&encode(&manifest)).expect("manifest parses");
        assert_eq!(root.name, "manifest");
        assert_eq!(root.attr_str("package"), Some("com.example.hello"));
        assert_eq!(root.attr_u32("versionCode"), Some(7));
        assert_eq!(root.attr_str("versionName"), Some("1.2"));

        let sdk = root.child("uses-sdk").unwrap();
        assert_eq!(sdk.attr_u32("minSdkVersion"), Some(21));
        assert_eq!(sdk.attr_u32("targetSdkVersion"), Some(34));

        let app = root.child("application").unwrap();
        assert_eq!(app.attr_str("label"), Some("Hello"));
        let activity = app.child("activity").unwrap();
        assert_eq!(activity.attr_str("name"), Some(".MainActivity"));
        assert_eq!(activity.attr_bool("exported"), Some(true));
        assert_eq!(activity.child("intent-filter").unwrap().children.len(), 2);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(b"not an apk").is_err());
        assert!(parse(&[3, 0, 8, 0]).is_err());
    }
}
