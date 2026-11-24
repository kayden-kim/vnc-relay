//! Safe registry access; startup never writes the Run value.

use crate::copy::Language;
use crate::rules::{self, Reason};
use std::path::Path;
use windows_registry::{CURRENT_USER, LOCAL_MACHINE};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE: &str = "vnc-relay-tray";

pub fn language() -> Language {
    for (path, name) in [
        (r"Control Panel\Desktop", "PreferredUILanguages"),
        (
            r"Control Panel\Desktop\MuiCached",
            "MachinePreferredUILanguages",
        ),
    ] {
        if let Ok(key) = CURRENT_USER.open(path)
            && let Ok(tags) = key.get_multi_string(name)
            && let Some(tag) = tags.first().filter(|tag| !tag.is_empty())
        {
            return rules::language_tag(tag);
        }
    }
    if LOCAL_MACHINE
        .open(r"SYSTEM\CurrentControlSet\Control\Nls\Language")
        .and_then(|key| key.get_string("InstallLanguage"))
        .is_ok_and(|value| value.eq_ignore_ascii_case("0412"))
    {
        Language::Korean
    } else {
        Language::English
    }
}

pub fn sign_in(executable: &Path) -> bool {
    CURRENT_USER
        .open(RUN)
        .and_then(|key| key.get_string(VALUE))
        .is_ok_and(|value| rules::run_value_matches(&value, &executable.to_string_lossy()))
}

pub fn toggle_sign_in(executable: &Path) -> Result<(), Reason> {
    let checked = sign_in(executable);
    let key = CURRENT_USER
        .create(RUN)
        .map_err(|error| registry_error(error.code().0))?;
    if checked {
        key.remove_value(VALUE)
    } else {
        key.set_string(VALUE, format!("\"{}\"", executable.display()))
    }
    .map_err(|error| registry_error(error.code().0))
}

fn registry_error(hresult: i32) -> Reason {
    let code = hresult & 0xffff;
    if code == 5 {
        Reason::Denied
    } else {
        Reason::Os(code)
    }
}
