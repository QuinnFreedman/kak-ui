//! Provides a high-level wrapper around [kakoune's JSON-RPC UI](https://github.com/mawww/kakoune/blob/master/doc/json_ui.asciidoc).
//! This crate doesn't have any opinions on how you choose to communicate with kakoune or how you choose to deserialize/serialize JSON
//! as long as it is supported by serde.
//!
//! The main types to look at here are [`IncomingRequest`] and [`OutgoingRequest`].
//!
//! # Examples
//!
//! Basic usage:
//!
//!```rust
//! use std::io::{BufRead, BufReader};
//! use std::process::{Command, Child, Stdio};
//! use kak_ui::{IncomingRequest, OutgoingRequest};
//!
//! let kak_child_process = Command::new("kak")
//!     .args(&["-ui", "json"])
//!     .stdout(Stdio::piped())
//!     .stdin(Stdio::piped())
//!     .spawn()
//!     .unwrap();
//!
//! let incoming_request: IncomingRequest = serde_json::from_str(
//!     &BufReader::new(kak_child_process.stdout.unwrap())
//!         .lines()
//!         .next()
//!         .unwrap()
//!         .unwrap(),
//! )
//! .unwrap();
//!
//! let outgoing_request = OutgoingRequest::Keys(vec!["<esc>:q<ret>".to_string()]);
//! serde_json::to_writer(kak_child_process.stdin.unwrap(), &outgoing_request).unwrap();
//!```

// TODO: Add links to kakoune docs

use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

/// A color in kakoune. Currently, this is a newtype wrapper around String.
#[derive(Debug, Clone)]
pub enum KakColor {
    RGB(String),
    RGBA(String),
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Purple,
    Cyan,
    White,
    Default,
}

impl<'de> Deserialize<'de> for KakColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ColorVisitor)
    }
}

struct ColorVisitor;

impl<'de> Visitor<'de> for ColorVisitor {
    type Value = KakColor;
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        formatter.write_str("a kakoune color")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match s {
            "black" => Ok(KakColor::Black),
            "red" => Ok(KakColor::Red),
            "green" => Ok(KakColor::Green),
            "yellow" => Ok(KakColor::Yellow),
            "blue" => Ok(KakColor::Blue),
            "purple" => Ok(KakColor::Purple),
            "cyan" => Ok(KakColor::Cyan),
            "white" => Ok(KakColor::White),
            "default" => Ok(KakColor::Default),
            x => {
                if &x[..4] == "rgb:" {
                    Ok(KakColor::RGB((&x[4..]).to_string()))
                } else if &x[..5] == "rgba:" {
                    Ok(KakColor::RGBA((&x[5..]).to_string()))
                } else {
                    Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(x),
                        &"black|red|green|yellow|blue|purple|cyan|white|default|rgb:HEX",
                    ))
                }
            }
        }
    }
}

/// An attribute in kakoune
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KakAttribute {
    Underline,
    Reverse,
    Blink,
    Bold,
    Dim,
    Italic,
    FinalFg,
    FinalBg,
    FinalAttr,
}

/// A kakoune face
#[derive(Debug, Clone, Deserialize)]
pub struct KakFace {
    pub fg: KakColor,
    pub bg: KakColor,
    pub attributes: Vec<KakAttribute>,
}

/// A kakoune atom
#[derive(Debug, Clone, Deserialize)]
pub struct KakAtom {
    pub face: KakFace,
    pub contents: String,
}

/// A [`Vec`] of [`KakAtom`]
pub type KakLine = Vec<KakAtom>;

/// A coordinate in kakoune
#[derive(Debug, Clone, Deserialize)]
pub struct KakCoord {
    pub line: u32,
    pub column: u32,
}

/// A incoming request. Recieve this from kakoune's stdout
#[derive(Debug, Clone)]
pub enum IncomingRequest {
    Draw {
        lines: Vec<KakLine>,
        default_face: KakFace,
        padding_face: KakFace,
    },
    DrawStatus {
        status_line: KakLine,
        mode_line: KakLine,
        default_face: KakFace,
    },
    MenuShow {
        items: Vec<KakLine>,
        anchor: KakCoord,
        selected_item_face: KakFace,
        menu_face: KakFace,
        style: String,
    },
    MenuSelect {
        selected: u32,
    },
    MenuHide,
    InfoShow {
        title: KakLine,
        content: Vec<KakLine>,
        anchor: KakCoord,
        face: KakFace,
        style: String,
    },
    InfoHide,
    SetCursor {
        mode: String,
        coord: KakCoord,
    },
    SetUiOptions {
        options: HashMap<String, String>,
    },
    Refresh {
        force: bool,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "method", content = "params")]
enum RawIncomingRequest {
    Draw(Vec<KakLine>, KakFace, KakFace),
    DrawStatus(KakLine, KakLine, KakFace),
    MenuShow(Vec<KakLine>, KakCoord, KakFace, KakFace, String),
    MenuSelect((u32,)),
    MenuHide([(); 0]),
    InfoShow(KakLine, Vec<KakLine>, KakCoord, KakFace, String),
    InfoHide([(); 0]),
    SetCursor(String, KakCoord),
    SetUiOptions((HashMap<String, String>,)),
    Refresh((bool,)),
}

impl<'de> Deserialize<'de> for IncomingRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(<JsonRpc<RawIncomingRequest>>::deserialize(deserializer)?
            .inner
            .into())
    }
}

impl From<RawIncomingRequest> for IncomingRequest {
    fn from(raw_request: RawIncomingRequest) -> Self {
        type Raw = RawIncomingRequest;
        type Processed = IncomingRequest;
        match raw_request {
            Raw::Draw(a, b, c) => Processed::Draw {
                lines: a,
                default_face: b,
                padding_face: c,
            },
            Raw::DrawStatus(a, b, c) => Processed::DrawStatus {
                status_line: a,
                mode_line: b,
                default_face: c,
            },
            Raw::MenuShow(a, b, c, d, e) => Processed::MenuShow {
                items: a,
                anchor: b,
                selected_item_face: c,
                menu_face: d,
                style: e,
            },
            Raw::MenuSelect((a,)) => Processed::MenuSelect { selected: a },
            Raw::MenuHide(_) => Processed::MenuHide,
            Raw::InfoShow(a, b, c, d, e) => Processed::InfoShow {
                title: a,
                content: b,
                anchor: c,
                face: d,
                style: e,
            },
            Raw::InfoHide(_) => Processed::InfoHide,
            Raw::SetCursor(a, b) => Processed::SetCursor { mode: a, coord: b },
            Raw::SetUiOptions((a,)) => Processed::SetUiOptions { options: a },
            Raw::Refresh((a,)) => Processed::Refresh { force: a },
        }
    }
}

/// A outgoing request. Input this to kakoune via stdin.
#[derive(Debug, Clone)]
pub enum OutgoingRequest {
    Keys(Vec<String>),
    Resize {
        rows: u32,
        columns: u32,
    },
    Scroll {
        amount: u32,
    },
    MouseMove {
        line: u32,
        column: u32,
    },
    MousePress {
        button: String,
        line: u32,
        column: u32,
    },
    MouseRelease {
        button: String,
        line: u32,
        column: u32,
    },
    MenuSelect {
        index: u32,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "method", content = "params")]
enum RawOutgoingRequest {
    Keys(Vec<String>),
    Resize(u32, u32),
    Scroll((u32,)),
    MouseMove(u32, u32),
    MousePress(String, u32, u32),
    MouseRelease(String, u32, u32),
    MenuSelect((u32,)),
}

impl Serialize for OutgoingRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        JsonRpc::new(RawOutgoingRequest::from(self.clone())).serialize(serializer)
    }
}

impl From<OutgoingRequest> for RawOutgoingRequest {
    fn from(request: OutgoingRequest) -> Self {
        type Raw = RawOutgoingRequest;
        type Processed = OutgoingRequest;
        match request {
            Processed::Keys(vec) => Raw::Keys(vec),
            Processed::Resize {
                rows: a,
                columns: b,
            } => Raw::Resize(a, b),
            Processed::Scroll { amount: a } => Raw::Scroll((a,)),
            Processed::MouseMove { line: a, column: b } => Raw::MouseMove(a, b),
            Processed::MousePress {
                button: a,
                line: b,
                column: c,
            } => Raw::MousePress(a, b, c),
            Processed::MouseRelease {
                button: a,
                line: b,
                column: c,
            } => Raw::MouseRelease(a, b, c),
            Processed::MenuSelect { index: a } => Raw::MenuSelect((a,)),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct JsonRpc<T> {
    jsonrpc: String,
    #[serde(flatten)]
    pub inner: T,
}

impl<T> JsonRpc<T> {
    fn new(inner: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            inner,
        }
    }
}
