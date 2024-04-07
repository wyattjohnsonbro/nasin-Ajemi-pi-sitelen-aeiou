use std::ffi::{OsStr, OsString};

use log::{debug, error};
use windows::{core::{Result, GUID}, Win32::{Foundation::{GetLastError, HINSTANCE}, System::LibraryLoader::GetModuleFileNameA}};

pub fn setup(dll_module: HINSTANCE) {
    unsafe { DLL_MODULE = Some(dll_module) };
}

// global variables
static mut DLL_MODULE: Option<HINSTANCE> = None;
pub fn dll_module() -> HINSTANCE {
    unsafe{ DLL_MODULE.unwrap() }
}

static mut DLL_PATH: Option<OsString> = None;
pub fn dll_path() -> Result<&'static OsStr> {
    if unsafe { DLL_PATH.as_ref() }.is_none() {
        let mut buf: Vec<u8> = vec![0;512];
        unsafe { GetModuleFileNameA(dll_module(), &mut buf) };
        let len = buf.iter().position(|byte| *byte == 0).unwrap();
        if len == 0 {
            let err = unsafe { GetLastError() };
            error!("Failed to find the dll path. {:?}", err);
            return Err(err.into());
        }
        buf.truncate(buf.iter().position(|byte| *byte == 0).unwrap());
        let path = unsafe{ OsString::from_encoded_bytes_unchecked(buf) };
        debug!("Found dll in {}", path.to_string_lossy());
        unsafe { DLL_PATH = Some(path) };
    }
    let path: &'static OsStr = unsafe{ DLL_PATH.as_ref() }.unwrap();
    Ok(path)
}

// registration stuff
pub const IME_NAME: &str = "Ajemi";
pub const IME_NAME_ASCII: &str = "Ajemi";
pub const IME_ID: GUID = GUID::from_u128(0xC93D3D59_2FAC_40E0_ABC6_A3658749E2FA);
pub const LANG_ID: u16 = 0x409; // en-US
pub const LANG_PROFILE_ID: GUID = GUID::from_u128(0xA411A7FC_A082_4B8A_8741_AA4A72613933);
pub const LANGBAR_ITEM_ID: GUID = GUID::from_u128(0x95288B2B_4D3B_4D4A_BF5B_9342E4F75E4D);
pub const DISPLAY_ATTR_ID: GUID = GUID::from_u128(0xE42647FB_4BF0_4570_9013_768487C5CAAE);
pub const LITE_TRAY_ICON_INDEX: u32 = 0;
pub const DARK_TRAY_ICON_INDEX: u32 = 1;
// customization
pub const CANDI_NUM: usize = 5;
pub const CANDI_INDEXES: [&str; CANDI_NUM] = ["1", "2", "3", "4", "5"];
pub const CANDI_INDEX_SUFFIX: &str = ". ";
pub const CANDI_INDEX_SUFFIX_MONO: &str = ".";
pub const PREEDIT_DELIMITER: &str = "'";
// included text
pub const DEFAULT_CONF: &str = include_str!("../res/conf.toml");
pub const SITELEN_SCHEMA: &str = include_str!("../res/schema/sitelen.schema");
pub const EMOJI_SCHEMA: &str = include_str!("../res/schema/emoji.schema");

