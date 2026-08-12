use super::*;

pub(crate) const APP_NAME: &str = "ChemSema";
pub(crate) const DOCUMENT_DISPLAY_NAME: &str = "ChemSema Document";
pub(crate) const PROG_ID: &str = "ChemSema.Document";
pub(crate) const VERSIONED_PROG_ID: &str = "ChemSema.Document.1";
pub(crate) const FILE_ASSOCIATION_PROG_IDS: [&str; 5] = [
    "ChemSema JSON Document",
    "ChemSema Compressed Document",
    "ChemDraw CDXML Document",
    "ChemDraw CDX Document",
    "MDL SDfile",
];
pub(crate) const CLSID_STRING: &str = "{CB69F54F-F21E-44DE-84FB-89D98FECE056}";
pub(crate) const CLIPBOARD_FORMAT_NATIVE: &str = "Native";
pub(crate) const OLE_STREAM_MANIFEST: &str = "ChemSemaManifest";
pub(crate) const OLE_STREAM_CONTENTS: &str = "CONTENTS";
pub(crate) const OLE_STREAM_OLE: &str = "\u{0001}Ole";
pub(crate) const OLE_STREAM_DOCUMENT: &str = "ChemSemaDocument";
pub(crate) const OLE_STREAM_SOURCE_CDXML: &str = "ChemSemaSourceCdxml";
pub(crate) const OLE_STREAM_PREVIEW_SVG: &str = "ChemSemaPreviewSvg";
pub(crate) const OLE_STREAM_PRESENTATION_EMF: &str = "\u{0002}OlePres001";
pub(crate) const OLE_STREAM_PRESENTATION_EMF_WORD: &str = "\u{0002}OlePres000";
pub(crate) const OLE_STREAM_OBJ_INFO: &str = "\u{0003}ObjInfo";
pub(crate) const OLE_STREAM_ENHANCED_PRINT: &str = "\u{0003}EPRINT";
pub(crate) const CLIPBOARD_FORMAT_EMBEDDED_OBJECT: &str = "Embedded Object";
pub(crate) const CLIPBOARD_FORMAT_EMBED_SOURCE: &str = "Embed Source";
pub(crate) const CLIPBOARD_FORMAT_OBJECT_DESCRIPTOR: &str = "Object Descriptor";
pub(crate) const CLIPBOARD_FORMAT_RTF: &str = "Rich Text Format";
pub(crate) const CLIPBOARD_FORMAT_HTML: &str = "HTML Format";
pub(crate) const FORMAT_CHEMSEMA_FRAGMENT: &str = "ChemSema Clipboard Fragment";
pub(crate) const FORMAT_CHEMSEMA_NATIVE: &str = "ChemSema Native Document";
pub(crate) const FORMAT_CHEMSEMA_DOCUMENT_JSON: &str = "ChemSema Document JSON";
pub(crate) const FORMAT_CHEMDRAW_INTERCHANGE: &str = "ChemDraw Interchange Format";
pub(crate) const FORMAT_CDXML_MIME: &str = "chemical/x-cdxml";
pub(crate) const FORMAT_SVG_MIME: &str = "image/svg+xml";
pub(crate) const FORMAT_SVG: &str = "SVG";
pub(crate) const CF_UNICODETEXT_FORMAT: u16 = 13;
pub(crate) const GMEM_MOVEABLE_FLAG: u32 = 0x0002;
pub(crate) const DEFAULT_OBJECT_WIDTH_HIMETRIC: i32 = 6000;
pub(crate) const DEFAULT_OBJECT_HEIGHT_HIMETRIC: i32 = 3000;
pub(crate) const HIMETRIC_PER_CM: f64 = 1000.0;
pub(crate) const EMF_LOGICAL_UNITS_PER_CSS_PX: f64 = 2.0;
pub(crate) const WORD_A4_BODY_WIDTH_CM: f64 = 21.0 - 2.0 * 3.18;
pub(crate) const MIN_OBJECT_EXTENT_HIMETRIC: i32 = 100;
pub(crate) const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;
pub(crate) const ASFW_ANY_PROCESS: u32 = u32::MAX;
pub(crate) const DESKTOP_DEV_SERVER_ADDR: &str = "127.0.0.1:8767";
pub(crate) const CHEMSEMA_DESKTOP_EXE_NAME: &str = "chemsema-desktop.exe";
pub(crate) const CHEMSEMA_DESKTOP_ENV: &str = "CHEMSEMA_DESKTOP_EXE";
pub(crate) const OLE_EDIT_SESSION_PREFIX: &str = "chemsema-ole-edit";
pub(crate) const OLE_EDIT_TIMER_ID: usize = 0xC0CC;
pub(crate) const OLE_EDIT_POLL_MS: u32 = 750;
pub(crate) const WM_TIMER: u32 = 0x0113;
pub(crate) const WM_OLE_EDIT_SESSION_CHANGED: u32 = 0x80CC;

pub(crate) const CLSID_CHEMSEMA_DOCUMENT: GUID = GUID {
    data1: 0xcb69f54f,
    data2: 0xf21e,
    data3: 0x44de,
    data4: [0x84, 0xfb, 0x89, 0xd9, 0x8f, 0xec, 0xe0, 0x56],
};

pub(crate) const IID_IUNKNOWN: GUID = GUID {
    data1: 0x00000000,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub(crate) const IID_ICLASS_FACTORY: GUID = GUID {
    data1: 0x00000001,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub(crate) const IID_IDATA_OBJECT: GUID = GUID {
    data1: 0x0000010e,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub(crate) const IID_IENUM_FORMATETC: GUID = GUID {
    data1: 0x00000103,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub(crate) const IID_IPERSIST: GUID = GUID {
    data1: 0x0000010c,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub(crate) const IID_IPERSIST_STORAGE: GUID = GUID {
    data1: 0x0000010a,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub(crate) const IID_IPERSIST_FILE: GUID = GUID {
    data1: 0x0000010b,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub(crate) const IID_IOLE_OBJECT: GUID = GUID {
    data1: 0x00000112,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub(crate) const IID_IVIEW_OBJECT: GUID = GUID {
    data1: 0x0000010d,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub(crate) const IID_IVIEW_OBJECT2: GUID = GUID {
    data1: 0x00000127,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub(crate) const IID_IRUNNABLE_OBJECT: GUID = GUID {
    data1: 0x00000126,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

pub(crate) const S_OK: i32 = 0;
pub(crate) const S_FALSE: i32 = 1;
pub(crate) const E_FAIL: i32 = 0x80004005u32 as i32;
pub(crate) const E_POINTER: i32 = 0x80004003u32 as i32;
pub(crate) const E_NOINTERFACE: i32 = 0x80004002u32 as i32;
pub(crate) const E_NOTIMPL: i32 = 0x80004001u32 as i32;
pub(crate) const E_OUTOFMEMORY: i32 = 0x8007000eu32 as i32;
pub(crate) const DV_E_FORMATETC: i32 = 0x80040064u32 as i32;
pub(crate) const DV_E_TYMED: i32 = 0x80040069u32 as i32;
pub(crate) const OLE_E_NOTRUNNING: i32 = 0x80040005u32 as i32;
pub(crate) const CLASS_E_NOAGGREGATION: i32 = 0x80040110u32 as i32;
