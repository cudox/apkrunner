//! Test-only builder for binary XML documents.
//!
//! Producing real AXML in tests lets us exercise the parser end-to-end
//! without shipping an APK fixture (which would be a large binary blob).

use crate::axml::{NO_ENTRY, RES_STRING_POOL_TYPE, RES_XML_END_ELEMENT_TYPE};
use crate::axml::{RES_XML_START_ELEMENT_TYPE, RES_XML_TYPE};

pub(crate) struct Xml {
    pub name: &'static str,
    pub attrs: Vec<(&'static str, AttrValue)>,
    pub children: Vec<Xml>,
}

pub(crate) enum AttrValue {
    Str(&'static str),
    Int(i32),
    Bool(bool),
    Ref(u32),
}

pub(crate) fn el(name: &'static str, attrs: Vec<(&'static str, AttrValue)>) -> Xml {
    Xml {
        name,
        attrs,
        children: Vec::new(),
    }
}

pub(crate) fn with(mut parent: Xml, children: Vec<Xml>) -> Xml {
    parent.children = children;
    parent
}

/// Serialize a small XML tree as an AXML document with a UTF-8 string pool.
pub(crate) fn encode(root: &Xml) -> Vec<u8> {
    let mut strings: Vec<String> = Vec::new();
    intern_tree(root, &mut strings);
    let mut body = Vec::new();
    encode_node(root, &strings, &mut body);

    let mut xml = Vec::new();
    xml.extend_from_slice(&RES_XML_TYPE.to_le_bytes());
    xml.extend_from_slice(&8u16.to_le_bytes());
    xml.extend_from_slice(&0u32.to_le_bytes()); // patched below
    xml.extend_from_slice(&string_pool(&strings));
    xml.extend_from_slice(&body);

    let size = xml.len() as u32;
    xml[4..8].copy_from_slice(&size.to_le_bytes());
    xml
}

fn intern(strings: &mut Vec<String>, s: &str) -> u32 {
    if let Some(i) = strings.iter().position(|x| x == s) {
        return i as u32;
    }
    strings.push(s.to_string());
    (strings.len() - 1) as u32
}

fn intern_tree(node: &Xml, strings: &mut Vec<String>) {
    intern(strings, node.name);
    for (name, value) in &node.attrs {
        intern(strings, name);
        if let AttrValue::Str(s) = value {
            intern(strings, s);
        }
    }
    for child in &node.children {
        intern_tree(child, strings);
    }
}

fn string_pool(strings: &[String]) -> Vec<u8> {
    let mut pool = Vec::new();
    let mut offsets = Vec::new();
    for s in strings {
        offsets.push(pool.len() as u32);
        let bytes = s.as_bytes();
        if bytes.len() < 0x80 {
            pool.push(bytes.len() as u8);
        } else {
            pool.push(((bytes.len() >> 8) as u8) | 0x80);
            pool.push((bytes.len() & 0xff) as u8);
        }
        pool.extend_from_slice(bytes);
        pool.push(0);
    }
    while pool.len() % 4 != 0 {
        pool.push(0);
    }

    let strings_start = 28 + offsets.len() * 4;
    let mut body = Vec::new();
    body.extend_from_slice(&(strings.len() as u32).to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // style count
    body.extend_from_slice(&0x100u32.to_le_bytes()); // UTF-8 flag
    body.extend_from_slice(&(strings_start as u32).to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // styles start
    for offset in &offsets {
        body.extend_from_slice(&offset.to_le_bytes());
    }
    body.extend_from_slice(&pool);
    chunk(RES_STRING_POOL_TYPE, 28, &body)
}

fn encode_node(node: &Xml, strings: &[String], out: &mut Vec<u8>) {
    let mut attrs = Vec::new();
    for (name, value) in &node.attrs {
        let (raw, data_type, data) = match value {
            AttrValue::Str(s) => {
                let index = index_of(strings, s);
                (index, 0x03u8, index)
            }
            AttrValue::Int(v) => (NO_ENTRY, 0x10u8, *v as u32),
            AttrValue::Bool(b) => (NO_ENTRY, 0x12u8, u32::from(*b)),
            AttrValue::Ref(id) => (NO_ENTRY, 0x01u8, *id),
        };
        attrs.push((NO_ENTRY, index_of(strings, name), raw, data_type, data));
    }

    let mut body = Vec::new();
    body.extend_from_slice(&NO_ENTRY.to_le_bytes()); // namespace
    body.extend_from_slice(&index_of(strings, node.name).to_le_bytes());
    body.extend_from_slice(&20u16.to_le_bytes()); // attribute start
    body.extend_from_slice(&20u16.to_le_bytes()); // attribute size
    body.extend_from_slice(&(attrs.len() as u16).to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes()); // id index
    body.extend_from_slice(&0u16.to_le_bytes()); // class index
    body.extend_from_slice(&0u16.to_le_bytes()); // style index
    for (namespace, name, raw, data_type, data) in attrs {
        body.extend_from_slice(&namespace.to_le_bytes());
        body.extend_from_slice(&name.to_le_bytes());
        body.extend_from_slice(&raw.to_le_bytes());
        body.extend_from_slice(&8u16.to_le_bytes());
        body.push(0); // res0
        body.push(data_type);
        body.extend_from_slice(&data.to_le_bytes());
    }
    // ResXMLTree_node prefix: line number + comment.
    body.splice(0..0, [0u8; 8]);

    out.extend_from_slice(&chunk(RES_XML_START_ELEMENT_TYPE, 36, &body));

    for child in &node.children {
        encode_node(child, strings, out);
    }

    let mut end_body = Vec::new();
    end_body.extend_from_slice(&[0u8; 8]); // line number + comment
    end_body.extend_from_slice(&NO_ENTRY.to_le_bytes()); // namespace
    end_body.extend_from_slice(&index_of(strings, node.name).to_le_bytes());
    out.extend_from_slice(&chunk(RES_XML_END_ELEMENT_TYPE, 16, &end_body));
}

fn chunk(kind: u16, header_size: u16, body: &[u8]) -> Vec<u8> {
    let size = (8 + body.len()) as u32;
    let mut out = Vec::with_capacity(size as usize);
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(&header_size.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(body);
    out
}

fn index_of(strings: &[String], s: &str) -> u32 {
    strings.iter().position(|x| x == s).unwrap() as u32
}

/// Build a minimal but valid APK (manifest + one dex) for integration tests.
///
/// Enabled by the `test-fixtures` feature so other crates can exercise the
/// CLI without committing a binary blob to the repository.
#[cfg(feature = "test-fixtures")]
pub fn minimal_apk() -> Vec<u8> {
    use std::io::Write;

    let manifest = with(
        el(
            "manifest",
            vec![
                ("package", AttrValue::Str("com.example.hello")),
                ("versionCode", AttrValue::Int(1)),
                ("versionName", AttrValue::Str("1.0")),
            ],
        ),
        vec![
            el("uses-sdk", vec![("minSdkVersion", AttrValue::Int(21))]),
            with(
                el(
                    "application",
                    vec![
                        ("label", AttrValue::Str("Hello")),
                        ("icon", AttrValue::Ref(0x7f02_0001)),
                    ],
                ),
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
                                vec![("name", AttrValue::Str("android.intent.category.LAUNCHER"))],
                            ),
                        ],
                    )],
                )],
            ),
        ],
    );

    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip.start_file("AndroidManifest.xml", options).unwrap();
        zip.write_all(&encode(&manifest)).unwrap();
        zip.start_file("classes.dex", options).unwrap();
        zip.write_all(&[0u8; 128]).unwrap();
        zip.finish().unwrap();
    }
    buffer.into_inner()
}
