//! Provides a high-level wrapper around [kakoune's JSON-RPC UI](https://github.com/mawww/kakoune/blob/master/doc/json_ui.asciidoc).
//!
//! The main types here are IncomingRequest and OutgoingRequest.


// TODO: Add links to kakoune docs
// TODO: Add example(s)

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

/// A color in kakoune. Currently, this is a newtype wrapper around String.
#[derive(Debug, Clone, Deserialize)]
pub struct KakColor(String);

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

// A kakoune face
#[derive(Debug, Clone, Deserialize)]
pub struct KakFace {
    fg: KakColor,
    bg: KakColor,
    attributes: Vec<KakAttribute>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KakAtom {
    face: KakFace,
    contents: String,
}

/// A [`Vec`] of [`KakAtom`]
pub type KakLine = Vec<KakAtom>;

/// A coordinate in kakoune
#[derive(Debug, Clone, Deserialize)]
pub struct KakCoord {
    line: u32,
    column: u32,
}

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
            Raw::SetCursor(a, b) => Processed::SetCursor {
                mode: a,
                coord: b,
            },
            Raw::SetUiOptions((a,)) => Processed::SetUiOptions { options: a },
            Raw::Refresh((a,)) => Processed::Refresh { force: a },
        }
    }
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

impl Serialize for OutgoingRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        JsonRpc::new(RawOutgoingRequest::from(self.clone())).serialize(serializer)
    }
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
