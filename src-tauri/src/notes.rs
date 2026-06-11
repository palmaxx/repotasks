use crate::models::{Entry, EntryKind};

/// Format a single Inbox line for a note or todo.
pub fn format_entry(text: &str, is_todo: bool, stamp: &str) -> String {
    let clean = clean_text(text);
    if is_todo {
        format!("- [ ] {stamp} — {clean}")
    } else {
        format!("- {stamp} — {clean}")
    }
}

/// Insert `entry_line` just beneath the `## Inbox` heading (so newest entries
/// sit at the top of the section), creating the section at the end of the file
/// if it is missing. Preserves a trailing newline when the input had one.
pub fn append_to_inbox(content: &str, entry_line: &str) -> String {
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    if let Some(idx) = lines.iter().position(|l| l.trim() == "## Inbox") {
        lines.insert(idx + 1, entry_line.to_string());
        let mut out = lines.join("\n");
        if content.ends_with('\n') {
            out.push('\n');
        }
        out
    } else {
        let mut out = content.trim_end().to_string();
        out.push_str("\n\n## Inbox\n");
        out.push_str(entry_line);
        out.push('\n');
        out
    }
}

/// Parse every bullet line in the file into a structured entry. Headings and
/// blank/free-text lines are ignored. Recognizes `- [ ]`/`- [x]` todos and
/// `- ` notes, with an optional leading `YYYY-MM-DD HH:MM` timestamp.
pub fn parse_notes(content: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    for (i, raw) in content.lines().enumerate() {
        let line = raw.trim_start();
        let (kind, done, rest) = if let Some(r) = line.strip_prefix("- [ ] ") {
            (EntryKind::Todo, false, r)
        } else if let Some(r) = line
            .strip_prefix("- [x] ")
            .or_else(|| line.strip_prefix("- [X] "))
        {
            (EntryKind::Todo, true, r)
        } else if let Some(r) = line.strip_prefix("- ") {
            (EntryKind::Note, false, r)
        } else {
            continue;
        };

        let (timestamp, text) = split_timestamp(rest);
        out.push(Entry {
            kind,
            done,
            text: text.trim().to_string(),
            timestamp,
            line: i,
        });
    }
    out
}

/// Flip a todo line between `- [ ]` and `- [x]`. Errors if the line is not a todo.
pub fn toggle_todo_at(content: &str, line: usize) -> Result<String, String> {
    let (mut lines, trailing) = split_lines(content);
    let original = lines.get(line).ok_or("Line out of range")?.clone();
    let trimmed = original.trim_start();
    let indent = &original[..original.len() - trimmed.len()];

    let new = if let Some(r) = trimmed.strip_prefix("- [ ] ") {
        format!("{indent}- [x] {r}")
    } else if let Some(r) = trimmed
        .strip_prefix("- [x] ")
        .or_else(|| trimmed.strip_prefix("- [X] "))
    {
        format!("{indent}- [ ] {r}")
    } else {
        return Err("Not a todo line".into());
    };

    lines[line] = new;
    Ok(join_lines(lines, trailing))
}

/// Replace the text of an entry line, keeping its bullet, checkbox state, and
/// any leading timestamp intact.
pub fn update_text_at(content: &str, line: usize, new_text: &str) -> Result<String, String> {
    let (mut lines, trailing) = split_lines(content);
    let original = lines.get(line).ok_or("Line out of range")?.clone();
    let trimmed = original.trim_start();
    let indent = &original[..original.len() - trimmed.len()];
    let clean = clean_text(new_text);

    let (marker, after) = if let Some(r) = trimmed.strip_prefix("- [ ] ") {
        ("- [ ] ", r)
    } else if let Some(r) = trimmed
        .strip_prefix("- [x] ")
        .or_else(|| trimmed.strip_prefix("- [X] "))
    {
        ("- [x] ", r)
    } else if let Some(r) = trimmed.strip_prefix("- ") {
        ("- ", r)
    } else {
        return Err("Not an entry line".into());
    };

    let (timestamp, _) = split_timestamp(after);
    let new_line = match timestamp {
        Some(t) => format!("{indent}{marker}{t} — {clean}"),
        None => format!("{indent}{marker}{clean}"),
    };

    lines[line] = new_line;
    Ok(join_lines(lines, trailing))
}

/// Remove an entry line entirely.
pub fn delete_at(content: &str, line: usize) -> Result<String, String> {
    let (mut lines, trailing) = split_lines(content);
    if line >= lines.len() {
        return Err("Line out of range".into());
    }
    lines.remove(line);
    Ok(join_lines(lines, trailing))
}

fn clean_text(text: &str) -> String {
    text.trim().replace('\n', " ").replace('\r', " ")
}

fn split_lines(content: &str) -> (Vec<String>, bool) {
    (
        content.lines().map(|l| l.to_string()).collect(),
        content.ends_with('\n'),
    )
}

fn join_lines(lines: Vec<String>, trailing: bool) -> String {
    let mut out = lines.join("\n");
    if trailing {
        out.push('\n');
    }
    out
}

/// Split a leading `YYYY-MM-DD HH:MM — ` timestamp off the front of a line.
fn split_timestamp(s: &str) -> (Option<String>, &str) {
    if let Some(pos) = s.find(" — ") {
        let (head, tail) = s.split_at(pos);
        if is_timestamp(head) {
            return (Some(head.to_string()), &tail[" — ".len()..]);
        }
    }
    (None, s)
}

fn is_timestamp(s: &str) -> bool {
    let b = s.as_bytes();
    s.len() == 16
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b' '
        && b[13] == b':'
        && s.chars()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7 | 10 | 13) || c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_note_and_todo() {
        assert_eq!(
            format_entry("hi there", false, "2026-06-11 10:00"),
            "- 2026-06-11 10:00 — hi there"
        );
        assert_eq!(
            format_entry("do it", true, "2026-06-11 10:00"),
            "- [ ] 2026-06-11 10:00 — do it"
        );
    }

    #[test]
    fn collapses_newlines_into_one_line() {
        assert_eq!(
            format_entry("line one\nline two", false, "t"),
            "- t — line one line two"
        );
    }

    #[test]
    fn inserts_under_inbox_newest_first() {
        let content = "# R — Notes\n\n## Inbox\n- old item\n\n## Notes\n";
        let out = append_to_inbox(content, "- new item");
        let inbox = out.find("## Inbox").unwrap();
        let new_pos = out.find("- new item").unwrap();
        let old_pos = out.find("- old item").unwrap();
        assert!(inbox < new_pos && new_pos < old_pos);
        assert!(out.ends_with('\n'));
        assert!(out.contains("## Notes"));
    }

    #[test]
    fn creates_inbox_when_missing() {
        let out = append_to_inbox("# R — Notes\n", "- x");
        assert!(out.contains("## Inbox"));
        assert!(out.contains("- x"));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn parses_todos_and_notes_with_timestamps() {
        let content = "# R\n\n## Inbox\n- [ ] 2026-06-11 10:00 — buy milk\n- [x] 2026-06-10 09:00 — done thing\n- 2026-06-11 11:00 — just a note\n- plain note no stamp\n## Notes\n";
        let entries = parse_notes(content);
        assert_eq!(entries.len(), 4);

        assert_eq!(entries[0].kind, EntryKind::Todo);
        assert!(!entries[0].done);
        assert_eq!(entries[0].text, "buy milk");
        assert_eq!(entries[0].timestamp.as_deref(), Some("2026-06-11 10:00"));
        assert_eq!(entries[0].line, 3);

        assert!(entries[1].done);

        assert_eq!(entries[2].kind, EntryKind::Note);
        assert_eq!(entries[2].text, "just a note");

        assert_eq!(entries[3].timestamp, None);
        assert_eq!(entries[3].text, "plain note no stamp");
    }

    #[test]
    fn does_not_mistake_emdash_in_text_for_timestamp() {
        let entries = parse_notes("- a thought — with an em dash\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].timestamp, None);
        assert_eq!(entries[0].text, "a thought — with an em dash");
    }

    #[test]
    fn toggles_todo_state_both_ways() {
        let c = "## Inbox\n- [ ] 2026-06-11 10:00 — task\n";
        let on = toggle_todo_at(c, 1).unwrap();
        assert!(on.contains("- [x] 2026-06-11 10:00 — task"));
        let off = toggle_todo_at(&on, 1).unwrap();
        assert!(off.contains("- [ ] 2026-06-11 10:00 — task"));
        assert!(off.ends_with('\n'));
    }

    #[test]
    fn toggle_rejects_non_todo_and_out_of_range() {
        assert!(toggle_todo_at("- a note\n", 0).is_err());
        assert!(toggle_todo_at("- [ ] x\n", 9).is_err());
    }

    #[test]
    fn updates_text_preserving_marker_and_timestamp() {
        assert_eq!(
            update_text_at("- [ ] 2026-06-11 10:00 — old\n", 0, "new text").unwrap(),
            "- [ ] 2026-06-11 10:00 — new text\n"
        );
        assert_eq!(
            update_text_at("- just a note\n", 0, "edited").unwrap(),
            "- edited\n"
        );
    }

    #[test]
    fn deletes_the_target_line_only() {
        assert_eq!(delete_at("a\nb\nc\n", 1).unwrap(), "a\nc\n");
    }
}
