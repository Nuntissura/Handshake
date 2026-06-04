use handshake_core::model_manual::{model_manual, projection::render_model_manual_markdown};

struct ManualProbe {
    question: &'static str,
    query_terms: &'static [&'static str],
    expected_evidence: &'static [&'static str],
}

fn answer_from_manual(manual_text: &str, query_terms: &[&str]) -> String {
    manual_text
        .lines()
        .filter(|line| {
            let lowercase = line.to_ascii_lowercase();
            query_terms
                .iter()
                .any(|term| lowercase.contains(&term.to_ascii_lowercase()))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn no_context_model_can_answer_core_operation_questions_from_model_manual() {
    let manual_markdown = render_model_manual_markdown(model_manual());
    let probes = [
        ManualProbe {
            question: "How should a fresh model discover the manual and command surface?",
            query_terms: &["startup", "kernel.model_manual.get", "command reference"],
            expected_evidence: &[
                "Read the ModelManual manifest through kernel.model_manual.get",
                "List command references",
            ],
        },
        ManualProbe {
            question: "How are HBR failures routed without relying on notes text?",
            query_terms: &["hbr", "violation", "receipt"],
            expected_evidence: &["typed HBR_VIOLATION receipts"],
        },
        ManualProbe {
            question: "Where does process lifecycle evidence land?",
            query_terms: &["process", "lifecycle", "Postgres"],
            expected_evidence: &["Postgres kernel_process_lifecycle rows"],
        },
        ManualProbe {
            question: "How should GUI diagnostics avoid interrupting the operator?",
            query_terms: &["diagnostics", "foreground", "focus"],
            expected_evidence: &["without foreground focus"],
        },
        ManualProbe {
            question: "What happens if a model tries to use a roadmap command?",
            query_terms: &["planned", "not yet callable"],
            expected_evidence: &["Command reference is planned and not yet callable"],
        },
    ];

    for probe in probes {
        let answer = answer_from_manual(&manual_markdown, probe.query_terms);
        assert!(
            !answer.trim().is_empty(),
            "manual produced no answer for probe: {}",
            probe.question
        );
        for expected in probe.expected_evidence {
            assert!(
                answer.contains(expected),
                "manual answer for probe `{}` did not include expected evidence `{}`.\nanswer:\n{}",
                probe.question,
                expected,
                answer
            );
        }
    }
}

#[test]
fn markdown_projection_has_machine_parseable_frontmatter_and_flat_topics() {
    let manual = model_manual();
    let markdown = render_model_manual_markdown(manual);

    assert!(markdown.starts_with("---\n"));
    assert!(markdown.contains("file_id: model-manual\n"));
    assert!(markdown.contains("file_kind: ModelManual\n"));
    assert!(markdown.contains("updated_at: \"2026-05-18T00:00:00Z\"\n"));
    assert!(markdown.contains(&format!("manual_version: \"{}\"\n", manual.version)));

    assert!(markdown.contains("<topic id=\"feature-hbr-process-diagnostics\""));
    assert!(markdown.contains("<topic id=\"command-model-manual-get\""));
    assert!(markdown.contains("<topic id=\"workflow-startup\""));
    assert!(markdown.contains("<topic id=\"safety-manual-no-context-operation\""));

    let opened = markdown.matches("<topic ").count();
    let closed = markdown.matches("</topic>").count();
    assert_eq!(opened, closed, "topic open/close counts differ");
    assert!(!markdown.contains("<topic id=\"nested"));
}
