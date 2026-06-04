//! Cloud Hypervisor resident warm-agent contract.
//!
//! The current persistent VM image contains a BusyBox serial command agent. That
//! agent proves snapshot/restore and generic exec, but it cannot keep llama.cpp
//! weights resident or emit live per-token frames. MT-207's warm path must only
//! advertise support once a guest image serves this contract over serial or
//! vsock.

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub const CLOUD_HYPERVISOR_WARM_AGENT_REQUIRED_TRANSPORT: &str =
    "model-bearing guest agent over serial/vsock";
pub const CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV: &str = "HANDSHAKE_CH_WARM_AGENT_HOST_PATH";
pub const CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT: &str = "/warm-agent";
pub const CLOUD_HYPERVISOR_WARM_AGENT_GUEST_PATH_METADATA_KEY: &str =
    "hsk.cloud_hypervisor.warm_agent_guest_path";
pub const CLOUD_HYPERVISOR_WARM_AGENT_CMDLINE_KEY: &str = "hsk.warm_agent";
pub const CLOUD_HYPERVISOR_WARM_AGENT_UNAVAILABLE_REASON: &str =
    "Cloud Hypervisor persistent VMs now expose a serial-socket command channel, \
     but warm-model RPC and live token streaming require a resident model-serving \
     guest agent/image; serial is the bootstrap transport and virtio-vsock remains \
     the hardened follow-on";
pub const CLOUD_HYPERVISOR_WARM_AGENT_PACKAGE_MANIFEST_FILE: &str = "hsk-warm-agent.package.json";
pub const CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE: &str = "llama-server";

const ELF64_HEADER_LEN: usize = 64;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u32 = 1;
const ET_EXEC: u16 = 2;
const ET_DYN: u16 = 3;
const EM_X86_64: u16 = 62;
const PT_INTERP: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudHypervisorWarmAgentContract {
    pub required_transport: String,
    pub host_path_env: String,
    pub guest_root: String,
    pub guest_path_metadata_key: String,
    pub required_protocol_id: String,
    pub required_protocol_version: u16,
    pub requires_model_residency: bool,
    pub requires_live_token_frames: bool,
    pub permits_shell_fallback: bool,
}

impl CloudHypervisorWarmAgentContract {
    pub fn current() -> Self {
        Self {
            required_transport: CLOUD_HYPERVISOR_WARM_AGENT_REQUIRED_TRANSPORT.to_string(),
            host_path_env: CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV.to_string(),
            guest_root: CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT.to_string(),
            guest_path_metadata_key: CLOUD_HYPERVISOR_WARM_AGENT_GUEST_PATH_METADATA_KEY
                .to_string(),
            required_protocol_id: crate::model_runtime::WARM_AGENT_PROTOCOL_ID.to_string(),
            required_protocol_version: crate::model_runtime::WARM_AGENT_PROTOCOL_VERSION,
            requires_model_residency: true,
            requires_live_token_frames: true,
            permits_shell_fallback: false,
        }
    }
}

pub fn warm_agent_unavailable_detail() -> String {
    let contract = CloudHypervisorWarmAgentContract::current();
    format!(
        "required_transport={}, protocol={}@v{}, requires_model_residency={}, \
         requires_live_token_frames={}, permits_shell_fallback={}, host_path_env={}, \
         guest_root={}, guest_path_metadata_key={}, reason={}",
        contract.required_transport,
        contract.required_protocol_id,
        contract.required_protocol_version,
        contract.requires_model_residency,
        contract.requires_live_token_frames,
        contract.permits_shell_fallback,
        contract.host_path_env,
        contract.guest_root,
        contract.guest_path_metadata_key,
        CLOUD_HYPERVISOR_WARM_AGENT_UNAVAILABLE_REASON
    )
}

pub fn validate_warm_agent_host_candidate(path: &Path) -> Result<(), String> {
    validate_static_x86_64_guest_elf(path, "warm-agent host path")?;
    validate_warm_agent_llama_server_sibling(path).map(|_| ())
}

pub fn validate_warm_agent_llama_server_sibling(agent_path: &Path) -> Result<PathBuf, String> {
    let parent = agent_path.parent().ok_or_else(|| {
        format!(
            "warm-agent host path has no parent directory for sibling {CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE}: {}",
            agent_path.display()
        )
    })?;
    let llama_server = parent.join(CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE);
    if !llama_server.is_file() {
        return Err(format!(
            "warm-agent package requires sibling {CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE} next to {} so the BusyBox guest can start resident model serving",
            agent_path.display()
        ));
    }
    validate_static_x86_64_guest_elf(&llama_server, "warm-agent companion llama-server")?;
    Ok(llama_server)
}

pub fn validate_static_x86_64_guest_elf(path: &Path, artifact_label: &str) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!(
            "{artifact_label} must point at a Linux guest executable file; missing: {}",
            path.display()
        ));
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("{artifact_label} must have a UTF-8 file name"))?;
    if file_name.chars().any(char::is_whitespace) {
        return Err(format!(
            "{artifact_label} file name must not contain whitespace: {file_name}"
        ));
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
    {
        return Err(format!(
            "{artifact_label} must be a Linux guest executable, not a Windows .exe: {}",
            path.display()
        ));
    }

    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|error| format!("{artifact_label} is unreadable: {error}"))?;
    if bytes.len() >= 2 && bytes[..2] == *b"MZ" {
        return Err(format!(
            "{artifact_label} has a Windows PE/MZ header; build/package a Linux ELF artifact before binding it into Cloud Hypervisor: {}",
            path.display()
        ));
    }
    validate_static_x86_64_elf_bytes(path, &bytes, artifact_label)
}

fn validate_static_x86_64_elf_bytes(
    path: &Path,
    bytes: &[u8],
    artifact_label: &str,
) -> Result<(), String> {
    if bytes.len() < ELF64_HEADER_LEN || bytes[..4] != *b"\x7fELF" {
        return Err(format!(
            "{artifact_label} must be a packaged static Linux x86_64 ELF guest artifact: {}",
            path.display()
        ));
    }
    if bytes[4] != ELFCLASS64 || bytes[5] != ELFDATA2LSB || bytes[6] != EV_CURRENT as u8 {
        return Err(format!(
            "{artifact_label} must be a 64-bit little-endian ELF guest artifact: {}",
            path.display()
        ));
    }
    let elf_type = read_u16_le(bytes, 16);
    if !matches!(elf_type, ET_EXEC | ET_DYN) {
        return Err(format!(
            "{artifact_label} must be an executable Linux ELF artifact: {}",
            path.display()
        ));
    }
    let machine = read_u16_le(bytes, 18);
    if machine != EM_X86_64 {
        return Err(format!(
            "{artifact_label} must target x86_64 for the Cloud Hypervisor guest, got ELF machine {machine}: {}",
            path.display()
        ));
    }
    let version = read_u32_le(bytes, 20);
    if version != EV_CURRENT {
        return Err(format!(
            "{artifact_label} has an unsupported ELF version {version}: {}",
            path.display()
        ));
    }

    let program_header_offset: usize = read_u64_le(bytes, 32)
        .try_into()
        .map_err(|_| "warm-agent ELF program-header offset overflows host usize".to_string())?;
    let program_header_entry_size = read_u16_le(bytes, 54) as usize;
    let program_header_count = read_u16_le(bytes, 56) as usize;
    if program_header_offset == 0 || program_header_entry_size < 4 || program_header_count == 0 {
        return Err(format!(
            "{artifact_label} must include ELF program headers for guest execution: {}",
            path.display()
        ));
    }
    let program_header_bytes = program_header_entry_size
        .checked_mul(program_header_count)
        .and_then(|len| program_header_offset.checked_add(len))
        .ok_or_else(|| "warm-agent ELF program-header table overflows host usize".to_string())?;
    if program_header_bytes > bytes.len() {
        return Err(format!(
            "{artifact_label} has a truncated ELF program-header table: {}",
            path.display()
        ));
    }
    for index in 0..program_header_count {
        let offset = program_header_offset + index * program_header_entry_size;
        if read_u32_le(bytes, offset) == PT_INTERP {
            return Err(format!(
                "{artifact_label} must be statically linked for the BusyBox initramfs guest; dynamic loader PT_INTERP found: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn read_u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_u64_le(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn contract_requires_resident_model_agent_and_rejects_shell_fallback() {
        let contract = CloudHypervisorWarmAgentContract::current();
        assert!(contract.requires_model_residency);
        assert!(contract.requires_live_token_frames);
        assert!(!contract.permits_shell_fallback);
        assert!(contract.required_transport.contains("serial"));
        assert!(contract.required_transport.contains("vsock"));
        assert_eq!(
            contract.host_path_env,
            CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV
        );
        assert_eq!(
            contract.guest_path_metadata_key,
            CLOUD_HYPERVISOR_WARM_AGENT_GUEST_PATH_METADATA_KEY
        );
    }

    #[test]
    fn host_candidate_rejects_windows_exe_and_mz_header() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let exe = dir.join("hsk-warm-agent.exe");
        std::fs::write(&exe, b"not even pe").expect("write exe");
        assert!(validate_warm_agent_host_candidate(&exe)
            .expect_err("exe extension rejects")
            .contains("Windows .exe"));

        let mz = dir.join("hsk-warm-agent");
        let mut file = std::fs::File::create(&mz).expect("create mz");
        file.write_all(b"MZ").expect("write mz");
        assert!(validate_warm_agent_host_candidate(&mz)
            .expect_err("mz header rejects")
            .contains("Windows PE/MZ"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_candidate_accepts_linux_elf_guest_artifact_name() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let agent = dir.join("hsk-warm-agent");
        std::fs::write(&agent, minimal_static_x86_64_elf()).expect("write elf");
        let llama_server = dir.join(CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE);
        std::fs::write(&llama_server, minimal_static_x86_64_elf()).expect("write llama-server elf");
        validate_warm_agent_host_candidate(&agent).expect("static x86_64 ELF candidate accepted");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_candidate_rejects_agent_without_sibling_llama_server() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let agent = dir.join("hsk-warm-agent");
        std::fs::write(&agent, minimal_static_x86_64_elf()).expect("write elf");
        assert!(validate_warm_agent_host_candidate(&agent)
            .expect_err("missing companion server must not pass production binding")
            .contains("requires sibling llama-server"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_candidate_rejects_non_static_sibling_llama_server() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let agent = dir.join("hsk-warm-agent");
        let llama_server = dir.join(CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE);
        std::fs::write(&agent, minimal_static_x86_64_elf()).expect("write elf");
        std::fs::write(&llama_server, b"#!/bin/sh\n").expect("write shell");
        assert!(validate_warm_agent_host_candidate(&agent)
            .expect_err("shell companion must not pass production binding")
            .contains("warm-agent companion llama-server"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_candidate_rejects_shell_script_even_without_pe_header() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let agent = dir.join("hsk-warm-agent");
        std::fs::write(&agent, b"#!/bin/sh\n").expect("write shell");
        assert!(validate_warm_agent_host_candidate(&agent)
            .expect_err("shell script must not advertise production warm support")
            .contains("static Linux x86_64 ELF"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_candidate_rejects_elf_magic_without_executable_guest_shape() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let agent = dir.join("hsk-warm-agent");
        std::fs::write(&agent, b"\x7fELFfake-test").expect("write malformed elf");
        assert!(validate_warm_agent_host_candidate(&agent)
            .expect_err("bare magic must not pass production binding")
            .contains("static Linux x86_64 ELF"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_candidate_rejects_dynamic_loader_elf() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let agent = dir.join("hsk-warm-agent");
        let mut elf = minimal_static_x86_64_elf();
        elf[64..68].copy_from_slice(&PT_INTERP.to_le_bytes());
        std::fs::write(&agent, elf).expect("write dynamic elf");
        assert!(validate_warm_agent_host_candidate(&agent)
            .expect_err("dynamic ELF must not pass production binding")
            .contains("statically linked"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn minimal_static_x86_64_elf() -> Vec<u8> {
        let mut elf = vec![0_u8; ELF64_HEADER_LEN + 56];
        elf[0..4].copy_from_slice(b"\x7fELF");
        elf[4] = ELFCLASS64;
        elf[5] = ELFDATA2LSB;
        elf[6] = EV_CURRENT as u8;
        elf[16..18].copy_from_slice(&ET_DYN.to_le_bytes());
        elf[18..20].copy_from_slice(&EM_X86_64.to_le_bytes());
        elf[20..24].copy_from_slice(&EV_CURRENT.to_le_bytes());
        elf[32..40].copy_from_slice(&(ELF64_HEADER_LEN as u64).to_le_bytes());
        elf[52..54].copy_from_slice(&(ELF64_HEADER_LEN as u16).to_le_bytes());
        elf[54..56].copy_from_slice(&56_u16.to_le_bytes());
        elf[56..58].copy_from_slice(&1_u16.to_le_bytes());
        elf[64..68].copy_from_slice(&1_u32.to_le_bytes());
        elf
    }
}
