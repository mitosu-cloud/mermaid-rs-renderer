---
name: describe-mermaid-jsonl
description: Fill the next <DESCRIPTION> placeholder in a JSONL training file whose entries contain chat-style role/content messages and Mermaid code in the assistant content. Use when Codex needs to inspect a file such as OUT.jsonl, find the next unresolved <DESCRIPTION> content field, describe what the assistant Mermaid diagram will draw in plain language, and update that JSON line safely.
---

# Describe Mermaid JSONL

## Workflow

Use `scripts/mermaid_jsonl_description.py` to locate and update the next unresolved entry.

1. Run the helper without a description:

```bash
python3 /path/to/describe-mermaid-jsonl/scripts/mermaid_jsonl_description.py /path/to/OUT.jsonl
```

2. Read the printed assistant `content`; that is the Mermaid code to describe.
3. Write one simple, accurate, plain-language description of what the diagram draws.
4. Replace the placeholder:

```bash
python3 /path/to/describe-mermaid-jsonl/scripts/mermaid_jsonl_description.py /path/to/OUT.jsonl --description "A flowchart showing ..."
```

If the target JSONL file is outside the writable workspace and the update command is blocked, rerun the same update command with the required filesystem approval.

## Description Rules

- Describe the rendered diagram, not the Mermaid syntax.
- Keep it short: usually one sentence, two only when the diagram is dense.
- Use plain language: "A flowchart showing...", "A sequence diagram where...", "A timeline of...".
- Mention the main labels, actors, stages, branches, or relationships that are visible in the code.
- Avoid inventing meaning that is not present in labels, arrows, or diagram structure.
- Avoid wording like "This Mermaid code..." or "The code creates...".
- Do not mention styling, colors, classes, or layout direction unless they materially change what the diagram communicates.

## Helper Behavior

The helper:

- Parses each JSONL line as JSON.
- Finds the first line where a `content` string contains `<DESCRIPTION>`.
- Prints the line number, placeholder path, assistant content path, and Mermaid code.
- Replaces only the first `<DESCRIPTION>` occurrence in the selected content field when `--description` is provided.
- Rewrites only the matched JSONL line and leaves all other lines byte-for-byte unchanged.
- Supports `--description-file` when quoting a description on the command line would be awkward.

Run the helper again after an update to move to the next remaining placeholder.
