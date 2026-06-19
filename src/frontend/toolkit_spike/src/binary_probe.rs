// Probe (d): single-binary dependency check.
// REAL self-inspection: parse THIS running executable's PE import table (via the `object` crate,
// because dumpbin is not on PATH on this host) and assert every imported DLL is an OS/system DLL
// — i.e. no bundled third-party .dll sits next to the .exe. The authoritative result is the
// release build (crt-static, fewest imports); in debug we still report the imports honestly.
//
// "System DLL" = a Windows apiset (api-ms-win-* / ext-ms-*), a CRT redistributable
// (vcruntime*/ucrtbase/msvcp*/msvcrt), or a DLL that resolves in %WINDIR%\System32. The contract
// explicitly lists VCRUNTIME* as an acceptable system DLL.

use object::Object;
use std::collections::BTreeSet;
use std::path::Path;

pub struct ProbeResult {
    pub pass: bool,
    pub notes: String,
}

fn is_system_dll(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("api-ms-win-") || lower.starts_with("ext-ms-") {
        return true;
    }
    for p in ["vcruntime", "ucrtbase", "msvcp", "msvcrt", "concrt"] {
        if lower.starts_with(p) {
            return true;
        }
    }
    if let Ok(windir) = std::env::var("WINDIR") {
        if Path::new(&windir).join("System32").join(name).exists() {
            return true;
        }
    }
    false
}

pub fn run() -> ProbeResult {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return ProbeResult { pass: false, notes: format!("current_exe failed: {e}") },
    };
    let data = match std::fs::read(&exe) {
        Ok(d) => d,
        Err(e) => return ProbeResult { pass: false, notes: format!("read exe failed: {e}") },
    };
    let file = match object::File::parse(&*data) {
        Ok(f) => f,
        Err(e) => return ProbeResult { pass: false, notes: format!("parse PE failed: {e}") },
    };
    let imports = match file.imports() {
        Ok(i) => i,
        Err(e) => return ProbeResult { pass: false, notes: format!("imports() failed: {e}") },
    };

    let mut dlls: BTreeSet<String> = BTreeSet::new();
    for imp in &imports {
        dlls.insert(String::from_utf8_lossy(imp.library()).to_string());
    }
    let nonsys: Vec<String> = dlls.iter().filter(|d| !is_system_dll(d)).cloned().collect();
    let profile = if cfg!(debug_assertions) { "debug" } else { "release(crt-static)" };
    let pass = nonsys.is_empty();

    ProbeResult {
        pass,
        notes: format!(
            "profile={profile}; {} imported DLLs (all system={}); non_system={:?}; dlls={:?}",
            dlls.len(),
            pass,
            nonsys,
            dlls
        ),
    }
}
