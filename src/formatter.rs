use memchr::memchr_iter;
use tree_sitter::Node;

type FResult<T> = Result<T, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum OutputToken<'a> {
    Leaf(String),
    InstructionStart,
    NewLine(usize),
    Space,
    Comma,
    Tab,
    Indent(usize),
    ResetIndent,
    SideStart,
    SideEnd,
    DelayStart,
    DelayEnd,
    LineComment(Node<'a>),
    BlockComment(Node<'a>),
}

#[derive(Default)]
struct OutputLine<'a> {
    text: String,
    side: Option<String>,
    delay: Option<String>,
    line_comment: Option<Node<'a>>,
    reading_side: bool,
    reading_delay: bool,
    is_instruction: bool,
}
impl<'a> OutputLine<'a> {
    fn push_text(&mut self, text: &str) {
        if self.reading_side {
            *self.side.get_or_insert_default() += text;
        } else if self.reading_delay {
            *self.delay.get_or_insert_default() += text;
        } else {
            self.text += text;
        }
    }

    fn push_space(&mut self) {
        let t = if self.reading_side {
            self.side.get_or_insert_default()
        } else if self.reading_delay {
            self.delay.get_or_insert_default()
        } else {
            &mut self.text
        };
        if !t.ends_with(' ') {
            t.push(' ');
        }
    }
}

struct Formatter<'a> {
    src: &'a str,
    out: Vec<OutputToken<'a>>,
    last: Node<'a>,
    last_non_comment: Node<'a>,
    bracket_depth: u32,
}
impl<'a> Formatter<'a> {
    fn new(src: &'a str, root: Node<'a>) -> Self {
        Self {
            src,
            out: Vec::new(),
            last: root,
            last_non_comment: root,
            bracket_depth: 0,
        }
    }

    fn emit_source(&mut self) {
        self.emit(self.last, None).unwrap();
    }

    fn finish(mut self) -> String {
        const TAB: &str = "    ";

        // ensure we end with a newline
        let last = self.out.last();
        match last {
            Some(OutputToken::NewLine(_)) => {}
            Some(_) => {
                // search for last newline token for line number
                let line_number = self
                    .out
                    .iter()
                    .rev()
                    .find_map(|e| match e {
                        OutputToken::NewLine(l) => Some(*l + 1),
                        _ => None,
                    })
                    .unwrap_or(0);
                self.push(OutputToken::NewLine(line_number));
            }
            None => self.push(OutputToken::NewLine(0)),
        }

        let mut indent = 0; // indent level of labels and instructions

        let mut output_lines = Vec::new();
        let mut cur_out: Option<OutputLine> = None;

        for e in &self.out {
            match e {
                OutputToken::Leaf(text) => cur_out.get_or_insert_default().push_text(text),
                OutputToken::InstructionStart => {
                    cur_out.get_or_insert_default().is_instruction = true
                }
                // OutputToken::Literal(s) => out += s,
                OutputToken::NewLine(_) => {
                    output_lines.push(cur_out.take().unwrap_or_default());
                    let new = cur_out.insert(OutputLine::default());
                    (0..indent).for_each(|_| new.text += TAB);
                }
                OutputToken::Space => cur_out.get_or_insert_default().push_space(),
                OutputToken::Comma => cur_out.get_or_insert_default().push_text(", "),
                OutputToken::Tab => cur_out.get_or_insert_default().push_text(TAB),
                OutputToken::Indent(ind) => indent = *ind,
                OutputToken::ResetIndent => indent = 0,
                OutputToken::BlockComment(node) => {
                    cur_out.get_or_insert_default().push_text(self.text(*node))
                }
                OutputToken::LineComment(node) => {
                    cur_out.get_or_insert_default().line_comment = Some(*node)
                }
                OutputToken::SideStart => {
                    debug_assert!(!cur_out.as_mut().unwrap().reading_delay);
                    debug_assert!(!cur_out.as_mut().unwrap().reading_side);

                    let last = cur_out.get_or_insert_default();
                    last.reading_side = true;
                    last.push_text("side ");
                }
                OutputToken::SideEnd => {
                    debug_assert!(cur_out.as_mut().unwrap().reading_side);
                    cur_out.as_mut().unwrap().reading_side = false;
                }
                OutputToken::DelayStart => {
                    debug_assert!(!cur_out.as_mut().unwrap().reading_delay);
                    debug_assert!(!cur_out.as_mut().unwrap().reading_side);

                    let last = cur_out.get_or_insert_default();
                    last.reading_delay = true;
                }
                OutputToken::DelayEnd => {
                    debug_assert!(cur_out.as_mut().unwrap().reading_delay);
                    cur_out.as_mut().unwrap().reading_delay = false;
                }
            }
        }
        assert!(cur_out.is_some()); // else newline is not last token

        // find max width of instructions
        let mut max_line_end = 0;
        for l in &output_lines {
            if !l.is_instruction {
                debug_assert!(l.side.is_none());
                debug_assert!(l.delay.is_none());
                continue;
            }
            max_line_end = max_line_end.max(l.text.len());
        }
        max_line_end += 1;

        // append side vertically aligned and compute new max width
        let mut max_line_end_side = max_line_end;
        for l in &mut output_lines {
            if !l.is_instruction {
                continue;
            }

            if let Some(side) = l.side.take() {
                for _ in 0..(max_line_end - l.text.len()) + 1 {
                    l.text.push(' ');
                }
                l.text += &side;
                max_line_end_side = max_line_end_side.max(l.text.len());
            }
        }

        // append delay vertically aligned and compute new max width
        let mut max_line_end_delay = max_line_end_side;
        for l in &mut output_lines {
            if !l.is_instruction {
                continue;
            }

            if let Some(delay) = l.delay.take() {
                for _ in 0..(max_line_end_side - l.text.len()) + 1 {
                    l.text.push(' ');
                }
                l.text += &delay;
                max_line_end_delay = max_line_end_delay.max(l.text.len());
            }
        }

        // append end of line comments vertically aligned
        for l in &mut output_lines {
            if let Some(line_comment) = l.line_comment.take() {
                let comment_text = self.text(line_comment);
                if !l.text.is_empty() {
                    let target_width = if l.is_instruction && comment_text.starts_with(';') {
                        max_line_end_delay + 1
                    } else {
                        // keep alignment if not after instruction or starts with //
                        line_comment.start_position().column
                    };

                    for _ in 0..(target_width.saturating_sub(l.text.len())) {
                        l.text.push(' ');
                    }
                }
                l.text += comment_text;
            }
        }

        // TODO: replace with writer api or preallocate
        let mut out_new = String::new();
        for e in output_lines {
            out_new += &e.text;
            out_new.push('\n');
        }
        out_new
    }

    fn text(&self, node: Node<'a>) -> &'a str {
        &self.src[node.byte_range()]
    }

    fn push(&mut self, value: OutputToken<'a>) {
        self.out.push(value);
    }

    fn newlines_between(&self, start: Node, end: Node) -> memchr::Memchr<'_> {
        let start = start.end_byte();
        let end = end.start_byte();
        memchr_iter(
            b'\n',
            self.src.as_bytes().get(start..end).unwrap_or_default(),
        )
    }

    fn newline(&mut self, node: Node<'a>, double: bool) {
        let line = node.start_position().row;
        if line == 0 {
            return;
        }

        self.push(OutputToken::NewLine(line - 1));

        if double || self.newlines_between(self.last, node).count() > 1 {
            self.push(OutputToken::NewLine(line - 1));
        }
    }

    fn emit(&mut self, node: Node<'a>, field_name: Option<&'a str>) -> FResult<()> {
        let last_kind = self.last.kind();

        // check if the node is right after an inline comment
        let after_inline_comment =
            last_kind == "block_comment" && self.newlines_between(self.last, node).next().is_none();

        // comma / spaces before words / numbers / etc in instructions
        if let Some(field_name) = field_name
            && match field_name {
                // | "irq_num" broken with irq n
                "gpio_num" | "pin_num" | "bit_count" => true,
                "value" if node.parent().map(|p| p.kind()) == Some("instr_set") => true,
                "op" if node.parent().map(|p| p.kind()) == Some("instr_mov") => true,
                // only include comma if we have a jmp condition
                "target" if last_kind == "jmp_condition" => true,
                _ => false,
            }
        {
            self.push(OutputToken::Comma);
        }

        log::debug!(
            "kind = {}, node = {node:?}, field_name = {:?}, children = {} ",
            node.kind(),
            field_name,
            node.child_count(),
        );

        // handle node kinds
        let kind = node.kind();
        let mut is_binary_expression = false;
        let mut is_side = false;
        let mut is_delay = false;
        match kind {
            "instruction" => {
                if !after_inline_comment {
                    self.newline(node, false);
                    self.push(OutputToken::Tab);
                }
                self.push(OutputToken::InstructionStart);
            }
            "side" => {
                self.push(OutputToken::SideStart);
                is_side = true;
            }
            "delay" => {
                self.push(OutputToken::DelayStart);
                is_delay = true;
            }
            "label" => {
                self.push(OutputToken::ResetIndent);

                let last_end = self.last_non_comment.end_byte();
                let node_start = node.start_byte();
                if node_start > last_end {
                    let last_to_node = &self.src[last_end..node_start];

                    let newline_before_label = last_to_node.bytes().rev().position(|e| e == b'\n');
                    if let Some(newline_before_label) = newline_before_label {
                        let mut indent =
                            &last_to_node[last_to_node.len() - 1 - newline_before_label + 1..];

                        let mut indent_level = 0;
                        // TODO: support tabs
                        const PAT: &str = "    ";
                        while let Some(i) = indent.find(PAT) {
                            indent_level += 1;
                            indent = &indent[i + PAT.len()..];
                        }
                        if indent_level != 0 {
                            self.push(OutputToken::Indent(indent_level));
                        }
                    }
                }

                if !after_inline_comment {
                    self.newline(node, false);
                }
            }
            "directive" => {
                self.push(OutputToken::ResetIndent);
                if !after_inline_comment {
                    self.newline(node, false);
                }
            }
            "code_block_start" => {
                self.push(OutputToken::ResetIndent);
                if !after_inline_comment {
                    self.newline(node, true);
                }
            }
            "," => {
                debug_assert_eq!(node.child_count(), 0);
                return Ok(());
            }
            "+" | "-" | "/" | "*" | "<<" | ">>"
                if let Some(p) = node.parent()
                    && p.kind() == "binary_expression" =>
            {
                is_binary_expression = true
            }
            "mov_source" => {
                // dont write comma if mov_op before source
                // with / without space between
                // TODO: find a better way to do this (this breaks if there is an inline comment in between)
                if !matches!(
                    self.out.len().checked_sub(2).and_then(|i| self.out.get(i)),
                    Some(OutputToken::Comma)
                ) && !matches!(
                    self.out.len().checked_sub(3).and_then(|i| self.out.get(i)),
                    Some(OutputToken::Comma)
                ) {
                    self.push(OutputToken::Comma);
                }
            }
            "comment" => {
                if self.newlines_between(self.last, node).next().is_some() {
                    self.push(OutputToken::NewLine(node.start_position().row));
                }
            }
            "line_comment" => {
                self.last = node;
                if !matches!(self.out.last(), None | Some(&OutputToken::NewLine(_))) {
                    self.push(OutputToken::Space);
                }
                self.push(OutputToken::LineComment(node));
                debug_assert_eq!(node.child_count(), 0);
                return Ok(());
            }
            "block_comment" => {
                self.last = node;
                self.push(OutputToken::Space);
                self.push(OutputToken::BlockComment(node));
                debug_assert_eq!(node.child_count(), 0);
                return Ok(());
            }
            "\n" => {
                debug_assert_eq!(node.child_count(), 0);
                return Ok(());
            }
            ":" | "source_file" | "code_block_end" => {} // don't add space before these
            "(" => self.bracket_depth += 1,
            ")" => self.bracket_depth = self.bracket_depth.saturating_sub(1),
            _ if self.bracket_depth != 0 => {
                dbg!(self.bracket_depth);
            }
            e => {
                log::debug!("unclassified = {e}");
                // no space after "." in directives (need all 3 conditions here)
                // dbg!(self.out.last());
                if !matches!(self.out.last(), Some(OutputToken::NewLine(_)))
                    && !e.starts_with("directive")
                    && e != "define_typ"
                    && !matches!(last_kind, ".")
                {
                    self.push(OutputToken::Space);
                }
            }
        }

        debug_assert!(!matches!(kind, "block_comment" | "line_comment"));

        // keep track of last and last non-comment nodes
        self.last = node;
        if kind != "comment" {
            self.last_non_comment = node;
        }

        // print leaf nodes as lower case
        let child_count = node.child_count();
        if child_count == 0 {
            let text = self.text(node);
            let text = if kind == "symbol" {
                text.to_string()
            } else {
                text.to_ascii_lowercase()
            };

            // spaces around binary operators
            if is_binary_expression {
                self.push(OutputToken::Space);
            }

            self.push(OutputToken::Leaf(text));

            // spaces around binary operators
            if is_binary_expression {
                self.push(OutputToken::Space);
            }
        } else {
            // swap side and delay children if delay is before side in an instruction
            let swap_side_delay = if kind == "instruction" {
                node.child_by_field_name("side")
                    .and_then(|side| Some((side, node.child_by_field_name("delay")?)))
                    .and_then(|e @ (side, delay)| {
                        (side.start_byte() > delay.start_byte()).then_some(e)
                    })
            } else {
                None
            };

            // process children
            // TODO: rewrite this function to be iterative and use a single cursor
            let mut cursor = node.walk();
            cursor.reset(node);
            cursor.goto_first_child();
            for _ in 0..child_count {
                let field_name = cursor.field_name();

                // swap side and delay if necessary, process children recursively
                match (swap_side_delay, field_name) {
                    (Some((_, delay)), Some("side")) => self.emit(delay, Some("delay"))?,
                    (Some((side, _)), Some("delay")) => self.emit(side, Some("side"))?,
                    _ => self.emit(cursor.node(), field_name)?,
                }

                cursor.goto_next_sibling();
            }
        }

        if is_side {
            self.out.push(OutputToken::SideEnd);
        }
        if is_delay {
            self.out.push(OutputToken::DelayEnd);
        }

        Ok(())
    }
}

pub fn format_tree(code: &str, root: Node) -> String {
    let mut fmt = Formatter::new(code, root);
    fmt.emit_source();
    fmt.finish()
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::{Path, PathBuf};

    use crate::{pio_parser, run_pioasm};

    fn find_all_pio_files(base_path: &Path, files: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(base_path).unwrap() {
            let e = e.unwrap();
            let meta = e.metadata().unwrap();
            if meta.is_dir() {
                find_all_pio_files(&e.path(), files);
            } else if meta.is_file() && e.file_name().to_str().unwrap().ends_with(".pio") {
                files.push(e.path());
            }
        }
    }

    #[test]
    fn format_pico_examples() {
        let mut pio_files = Vec::new();
        find_all_pio_files(Path::new("tests/pico-examples"), &mut pio_files);

        for f in pio_files {
            eprintln!("formatting {}", f.display());

            let code = std::fs::read_to_string(f).unwrap();

            let reference = run_pioasm(&code, None).unwrap();
            assert!(reference.status.success());
            assert!(reference.stderr.is_empty());

            let formatted =
                format_tree(&code, pio_parser().parse(&code, None).unwrap().root_node());

            let assembled = run_pioasm(&formatted, None).unwrap();

            assert!(
                assembled.stderr.is_empty(),
                "{}",
                String::from_utf8_lossy(&assembled.stderr)
            );
            assert!(assembled.status.success());

            assert_eq!(
                String::from_utf8(reference.stdout).unwrap(),
                String::from_utf8(assembled.stdout).unwrap(),
            );
        }
    }
}
