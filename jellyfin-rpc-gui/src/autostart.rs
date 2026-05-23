use winreg::enums::*;
use winreg::RegKey;

const REG_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APP_NAME: &str = "JellyfinRPC";

pub fn enable() -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy().to_string();
    let value = format!("\"{exe_str}\"");

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run_key, _) = hkcu.create_subkey(REG_PATH)?;
    run_key.set_value(APP_NAME, &value)?;
    Ok(())
}

pub fn disable() -> std::io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu.open_subkey_with_flags(REG_PATH, KEY_WRITE)?;
    match run_key.delete_value(APP_NAME) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn is_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(run_key) = hkcu.open_subkey(REG_PATH) else {
        return false;
    };
    run_key.get_value::<String, _>(APP_NAME).is_ok()
}

pub fn sync(desired: bool) -> std::io::Result<()> {
    let current = is_enabled();
    if desired == current {
        return Ok(());
    }
    if desired {
        enable()
    } else {
        disable()
    }
}
