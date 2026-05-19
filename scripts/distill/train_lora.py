#!/usr/bin/env python3
"""
MT-122: Teacher/student PEFT LoRA distillation trainer.

Invoked by the Rust orchestrator in
`handshake_core::distillation::peft_pipeline` once the corpus has
passed `ContentReview` and been written to JSONL. This script is the
out-of-process boundary because PEFT/Transformers training has no
production-ready Rust path as of the WP-KERNEL-004 window.

Operator-content discipline (GLOBAL-PRODUCTION-002..009): this script
does NOT filter, censor, moralise, or reword corpus content. The
upstream `ContentReview` gate handles PII + license + dedup; this
script's responsibility is the actual PEFT training run.

This file is intentionally minimal scaffolding. The orchestrator's
`PeftTrainerExecutor` trait abstracts subprocess launch + sandbox +
process-ledger registration; the production executor wires this
script under a SandboxAdapter (cluster B) and a
ProcessOwnershipLedger row with engine_kind=DistillationJob (MT-069).

Usage:
  python train_lora.py \
    --corpus <jsonl-path> \
    --teacher <teacher-model-path> \
    --student <student-base-model-path> \
    --out <lora-output-dir> \
    --rank 16 --alpha 32 --dropout 0.05 \
    --epochs 1 --lr 2e-4 --batch-size 4

Exit codes:
  0  - training succeeded; LoRA artifact written under <out>.
  1  - argument parsing failure.
  2  - corpus read failure.
  3  - dependency import failure (peft / transformers / torch missing).
  4  - training step failure.

When transformers/peft/torch are not installed (typical for the
WP-KERNEL-004 build host until MT-074 unblocks the native ML
toolchain), this script exits 3 with a clear operator-action hint so
the Rust executor can surface a `DistillError::TrainerUnavailable`.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="MT-122 PEFT LoRA distillation trainer")
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--teacher", required=True, type=Path)
    parser.add_argument("--student", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--rank", type=int, default=16)
    parser.add_argument("--alpha", type=float, default=32.0)
    parser.add_argument("--dropout", type=float, default=0.05)
    parser.add_argument("--epochs", type=int, default=1)
    parser.add_argument("--lr", type=float, default=2e-4)
    parser.add_argument("--batch-size", type=int, default=4)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    try:
        args = parse_args(argv)
    except SystemExit as exit_status:
        # argparse exits 2 on parsing errors; remap to 1 for the Rust
        # executor's expected codes.
        return 1 if exit_status.code in (None, 0) else 1

    if not args.corpus.is_file():
        print(
            f"MT-122 train_lora: corpus not found at {args.corpus}",
            file=sys.stderr,
        )
        return 2

    # Sanity-check the corpus JSONL shape so the executor surfaces a
    # clear error before bothering with imports.
    try:
        with args.corpus.open("r", encoding="utf-8") as handle:
            for line_no, raw in enumerate(handle, start=1):
                raw = raw.strip()
                if not raw:
                    continue
                obj = json.loads(raw)
                for key in ("prompt", "completion"):
                    if key not in obj:
                        print(
                            f"MT-122 train_lora: corpus line {line_no} missing key {key!r}",
                            file=sys.stderr,
                        )
                        return 2
    except (OSError, json.JSONDecodeError) as err:
        print(f"MT-122 train_lora: corpus read failed: {err}", file=sys.stderr)
        return 2

    try:
        # The actual training body. Imports are inside the try so a
        # missing-dependency host exits 3 cleanly and the Rust
        # executor maps to DistillError::TrainerUnavailable.
        import torch  # noqa: F401
        import transformers  # noqa: F401
        import peft  # noqa: F401
    except ImportError as err:
        print(
            "MT-122 train_lora: required Python dependencies not installed "
            f"({err}). Install scripts/distill/requirements.txt under your "
            "Handshake-managed Python environment.",
            file=sys.stderr,
        )
        return 3

    # ---------------------------------------------------------------
    # Production training body (deferred to a follow-on MT alongside
    # the cluster-B sandbox + MT-069 process ledger executor wiring).
    # ---------------------------------------------------------------
    #
    # The full implementation, once the dependencies above import, is:
    #   1. teacher_tokenizer + teacher_model = transformers.AutoModel.
    #      from_pretrained(args.teacher).
    #   2. student_tokenizer + student_model = transformers.AutoModel.
    #      from_pretrained(args.student) wrapped in
    #      peft.LoraConfig(r=args.rank, lora_alpha=args.alpha,
    #      lora_dropout=args.dropout).
    #   3. dataset = load_jsonl(args.corpus) -> tokenized -> DataLoader.
    #   4. trainer = transformers.Trainer(model=student_model, ...).
    #   5. trainer.train() for args.epochs at args.lr.
    #   6. student_model.save_pretrained(args.out).
    #
    # This script is intentionally stub-only in this MT to keep the
    # surface unit-testable (arg parsing + corpus shape validation +
    # dependency-detection exit codes); the executor abstraction in
    # the Rust orchestrator delegates the actual training run.
    args.out.mkdir(parents=True, exist_ok=True)
    print(
        f"MT-122 train_lora: scaffolding only - training body deferred. "
        f"corpus={args.corpus} teacher={args.teacher} student={args.student} "
        f"out={args.out} rank={args.rank} epochs={args.epochs}",
        file=sys.stderr,
    )
    return 4


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
