"""Read-only check for stray cargo output and builds (Operator 2026-11-07: "make sure no stray cargo builds happen"; HSK-ART-004).

Usage: python stray_cargo.py [--brief]
Reports repo-local target folders in the kernel and Studio worktrees, Handshake_Artifacts folders outside the one root,
and running cargo or rustc processes in the Studio tree whose output is outside the assigned roots. Deletes nothing.
--brief prints one line, for lane_watch.sh: `none`, or the counts and first paths.
"""
import json
import os
import re
import subprocess
import sys

S = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
KERNEL = os.path.abspath(os.path.join(S, "..", "..", ".."))
WORKTREES = os.path.dirname(KERNEL)
LANES = os.path.join(WORKTREES, "wt-studio-lanes")
ART = (os.environ.get("HANDSHAKE_ARTIFACTS_ROOT") or os.path.join(WORKTREES, "Handshake_Artifacts")).replace("\\", "/").lower()
ALLOWED = (ART + "/wp-kernel-studio/", "c:/handshakebuild/wp-kernel-studio/")


def local_targets():
    roots = [KERNEL, os.path.join(WORKTREES, "wt-studio-assets")]
    if os.path.isdir(LANES):
        roots += [e.path for e in os.scandir(LANES) if e.is_dir()]
    out = []
    for r in roots:
        for sub in ("target", os.path.join("handshake-studio-assets", "target")):
            p = os.path.join(r, sub)
            # nextest writes its store at <workspace>/target/nextest by default: small junit output, no compiled build.
            # Reported apart so that a real local build still stands out.
            if os.path.isdir(p) and os.listdir(p) != ["nextest"]:
                out.append(p.replace("\\", "/"))
    return out


def stray_artifact_folders():
    out = []
    for e in os.scandir(WORKTREES):
        if not e.is_dir() or e.name == "Handshake_Artifacts":
            continue
        for sub in ("Handshake_Artifacts", os.path.join("handshake-studio-assets", "Handshake_Artifacts")):
            p = os.path.join(e.path, sub)
            if os.path.isdir(p):
                out.append(p.replace("\\", "/"))
    if os.path.isdir(LANES):
        for e in os.scandir(LANES):
            for sub in ("Handshake_Artifacts", os.path.join("handshake-studio-assets", "Handshake_Artifacts")):
                p = os.path.join(e.path, sub)
                if os.path.isdir(p):
                    out.append(p.replace("\\", "/"))
    return out


def stray_processes():
    cmd = ("Get-CimInstance Win32_Process | Where-Object { $_.Name -in 'rustc.exe','cargo.exe','cargo-nextest.exe','clippy-driver.exe' } "
           "| ForEach-Object { [string]$_.ProcessId + ' ' + $_.CommandLine }")
    text = subprocess.run(["powershell", "-NoProfile", "-Command", cmd], capture_output=True, text=True).stdout
    out = []
    for line in text.splitlines():
        low = line.replace("\\", "/").lower()
        m = re.search(r"--out-dir[= ]\"?([^\" ]+(?: [^\" -][^\" ]*)*)", low)
        if not m:
            continue
        where = m.group(1)
        studio = "hsk_studio" in low or "handshake worktrees" in where or "handshake-studio-assets" in low
        if studio and not where.startswith(ALLOWED):
            out.append({"pid": line.split(" ", 1)[0], "out_dir": where[:160]})
    return out


def main():
    report = {"local_targets": local_targets(), "stray_artifact_folders": stray_artifact_folders(), "stray_processes": stray_processes()}
    if "--brief" in sys.argv:
        n = sum(len(v) for v in report.values())
        if not n:
            print("none")
        else:
            first = (report["stray_processes"] and report["stray_processes"][0]["out_dir"]) or (report["local_targets"] + report["stray_artifact_folders"])[0]
            print(f"{len(report['local_targets'])} local targets, {len(report['stray_artifact_folders'])} artifact folders, {len(report['stray_processes'])} processes; first {first[-70:]}")
    else:
        print(json.dumps(report, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
