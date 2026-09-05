use crate::{formatter::format_tree, pio_parser, pio_query, run_pioasm};
use foldhash::{HashMap, HashMapExt};
use gen_lsp_types::*;
use lsp_server::{Connection, Message, Request as ServerRequest, RequestId, Response};
use regex::Regex;
use std::borrow::Cow;
use std::fmt::Write;
use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::LazyLock,
};
use tree_sitter::{Node, QueryCursor, StreamingIterator};
use yoke::{Yoke, Yokeable};

/// convert treesitter Point to LSP Position (UTF-8 encoding only)
const fn ts_point_to_lsp(p: tree_sitter::Point) -> Position {
    Position::new(p.row as _, p.column as _)
}

/// convert LSP Position to treesitter Point (UTF-8 encoding only)
const fn lsp_pos_to_ts(p: Position) -> tree_sitter::Point {
    tree_sitter::Point::new(p.line as _, p.character as _)
}

/// convert treesitter Range to LSP Range (UTF-8 encoding only)
const fn ts_range_to_lsp(r: tree_sitter::Range) -> Range {
    Range::new(ts_point_to_lsp(r.start_point), ts_point_to_lsp(r.end_point))
}

/// find the field name in the parent of a Node
fn get_named_node_field_name<'a>(node: Node<'a>) -> Option<&'a str> {
    let parent = node.parent()?;

    let mut cursor = parent.walk();
    cursor.reset(parent);
    cursor.goto_first_child();
    for _ in 0..parent.child_count() {
        if cursor.node() == node {
            // found node in parents children
            return cursor.field_name();
        }
        cursor.goto_next_sibling();
    }
    None
}

/// check if a node is (part of) a label definition or reference
fn is_label(node: Node) -> bool {
    let mut cur = node;
    while let Some(p) = cur.parent() {
        if matches!(p.kind(), "label_reference" | "label") {
            return true;
        }
        cur = p;
    }
    false
}

/// check if a node is (part of) an instruction and return all nodes in the path to the instruction Node
fn get_instruction(node: Node) -> Option<Vec<Node>> {
    let mut cur = node;
    let mut path = vec![cur];
    while let Some(p) = cur.parent() {
        path.push(p);
        if p.kind() == "instruction" {
            return Some(path);
        }
        cur = p;
    }
    None
}

#[derive(Debug, PartialEq)]
pub struct NamedSymbol<'a> {
    /// name of the definition (e.g., "foo" for "foo:" or ".define foo 32")
    pub text: &'a str,
    /// range of the name (same as above, as LSP type)
    pub name_range: Range,
}

/// label or .define
#[derive(Debug, PartialEq)]
pub enum SymbolType<'a> {
    Define {
        value: &'a str,
        named: NamedSymbol<'a>,
    },
    Label(NamedSymbol<'a>),
    Wrap,
}

// value and position for .define and <label>:
#[derive(Debug)]
pub struct SymbolDefinition<'a> {
    /// range of the full statement (e.g., "foo:" or ".define foo 32")
    pub statement_range: Range,
    /// the program that this definition is part of
    pub program: Option<&'a str>,
    /// label, .define, .wrap, .wrap_target
    pub typ: SymbolType<'a>,
}

impl<'a> SymbolDefinition<'a> {
    pub const fn text(&self) -> Option<&str> {
        match &self.typ {
            SymbolType::Define { value: _, named } => Some(named.text),
            SymbolType::Label(named) => Some(named.text),
            SymbolType::Wrap => None,
        }
    }
    pub const fn name_range(&self) -> Option<Range> {
        match &self.typ {
            SymbolType::Define { value: _, named } => Some(named.name_range),
            SymbolType::Label(named) => Some(named.name_range),
            SymbolType::Wrap => None,
        }
    }
}

/// ts declaration of define (.define)
#[derive(Debug)]
pub struct ProgramDefine {
    /// range of the full statement (e.g., ".define foo 32")
    pub statement_range: tree_sitter::Range,
    /// range of the name (e.g., "foo" for ".define foo 32")
    pub name_range: tree_sitter::Range,
    /// range of the value (e.g., "32" for ".define foo 32")
    pub value_range: tree_sitter::Range,
}

/// ts declaration of label (<label>:)
#[derive(Debug)]
pub struct ProgramLabel {
    /// range of the full statement (e.g., "foo:")
    pub statement_range: tree_sitter::Range,
    /// name of the definition (e.g., "foo" for "foo:")
    pub name_range: tree_sitter::Range,
    /// index of the instruction the label is referencing
    pub instr_pc: u8,
    /// all references to this label
    pub references: Vec<Range>,
}

/// ts declaration of program (.program)
#[derive(Debug, Default)]
pub struct ProgramInfo<'a> {
    /// range of the full statement (e.g., ".program foo")
    pub statement_range: Range,
    /// number of instructions
    pub instr_count: u8,
    /// defines indexed by their name
    pub defines: HashMap<&'a str, ProgramDefine>,
    /// defines indexed by their name
    pub labels: HashMap<&'a str, ProgramLabel>,
    /// location of .wrap
    pub wrap: Option<Range>,
    /// location of .wrap_target
    pub wrap_target: Option<Range>,
}
impl<'a> ProgramInfo<'a> {
    fn new(statement_range: Range) -> Self {
        Self {
            statement_range,
            ..Default::default()
        }
    }

    /// instruction count inlay hint
    fn prog_hint(&self) -> InlayHint {
        InlayHint {
            position: self.statement_range.end,
            label: Label::String(format!("{} instructions", self.instr_count)),
            kind: Some(InlayHintKind::Parameter),
            padding_left: Some(true),
            padding_right: Some(false),
            text_edits: None,
            tooltip: None,
            data: None,
        }
    }
}

/// programs and global defines
#[derive(Yokeable, Default)]
pub struct Programs<'a> {
    /// programs in file index my name
    pub programs: HashMap<&'a str, ProgramInfo<'a>>,
    /// defines before the first program
    pub global_defines: HashMap<&'a str, ProgramDefine>,
}

/// loaded pio file
pub struct DocumentData {
    /// parsed tree-sitter tree
    pub tree: tree_sitter::Tree,
    /// active inlay hints
    inlay_hints: Vec<InlayHint>,
    /// programs and global defines, use Yoke so the struct can be self referential and avoid cloning
    pub programs: Yoke<Programs<'static>, String>,

    cursor: QueryCursor,
}
impl DocumentData {
    fn default_yoke() -> Yoke<Programs<'static>, String> {
        Yoke::attach_to_cart(String::new(), |_| Programs::default())
    }

    pub fn new(parser: &mut tree_sitter::Parser, text: String) -> Self {
        let mut this = Self {
            programs: Self::default_yoke(),
            tree: parser.parse("", None).unwrap(),
            inlay_hints: Vec::new(),
            cursor: QueryCursor::new(),
        };
        this.update(parser, text, None);
        this
    }

    /// update doc content and reanalyze
    pub fn update(
        &mut self,
        parser: &mut tree_sitter::Parser,
        text: String,
        old_tree: Option<&tree_sitter::Tree>,
    ) {
        self.tree = parser.parse(&text, old_tree).unwrap();

        let programs = self.programs.get();
        let size_hint = (programs.programs.len(), programs.global_defines.len());

        // reanalyze
        self.programs = Yoke::attach_to_cart(text, |text| self.analyze_programs(text, size_hint));
    }

    /// find node at the cursor position
    pub fn node_at(&self, position: Position) -> Option<Node<'_>> {
        let p = lsp_pos_to_ts(position);
        self.tree.root_node().descendant_for_point_range(p, p)
    }

    /// find the first program before a node. cur must be direct child of "source_file"
    fn first_program_before_root_node(&self, mut cur: Node) -> Option<&str> {
        while let Some(pr) = cur.prev_named_sibling() {
            if pr.kind() == "directive" {
                let mut cur = pr.walk();
                for c in pr.children(&mut cur) {
                    if c.kind() == "directive_program" {
                        let prog_name = c
                            .child_by_field_name("program_name")
                            .expect("no program_name field");
                        return Some(&self.programs.backing_cart()[prog_name.byte_range()]);
                    }
                }
            }
            cur = pr;
        }
        None
    }

    /// find the program that a node is part of
    fn program_at(&self, node: Node) -> Option<&str> {
        // walk up to root level and iterate back until finding a program definition
        let mut cur = node;
        while let Some(p) = cur.parent() {
            if p.kind() == "source_file" {
                return self.first_program_before_root_node(cur);
            }
            cur = p;
        }
        log::error!("no source_file Node found");
        None
    }

    /// find definition of a .define statement or label
    pub fn get_definition<'a>(&'a self, node: Node) -> Option<SymbolDefinition<'a>> {
        let kind = node.kind();
        match kind {
            "symbol" => {}
            "directive_wrap" | "directive_wrap_target" => {
                let prog_name = self.program_at(node.parent()?)?;
                let program = self.programs.get().programs.get(prog_name)?;
                return Some(SymbolDefinition {
                    statement_range: program.wrap_target?,

                    // statement_range: match kind {
                    //     "directive_wrap" => program.wrap?,
                    //     _ => program.wrap_target?,
                    // },
                    program: Some(prog_name),
                    typ: SymbolType::Wrap,
                });
            }
            _ => return None,
        }

        log::debug!("searching for definition of {node}");

        // check if symbol can be label reference
        let is_label = is_label(node);
        log::debug!("is_label = {is_label}");

        // the program that this symbol is part of
        let mut program = self.program_at(node);

        let doc_text = self.programs.backing_cart();
        let defs = self.programs.get();

        // symbol name
        let node_text = &doc_text[node.byte_range()];

        // look up program info for program
        let prog = program.and_then(|p| defs.programs.get(p));

        // search for label if jmp address (could still be define otherwise)
        if is_label && let Some(label) = prog.and_then(|p| p.labels.get(node_text)) {
            log::debug!("found label");
            return Some(SymbolDefinition {
                statement_range: ts_range_to_lsp(label.statement_range),
                program,
                typ: SymbolType::Label(NamedSymbol {
                    text: node_text,
                    name_range: ts_range_to_lsp(label.name_range),
                }),
            });
        }

        // look first for local and then for global definition
        let pd = prog
            .map(|e| &e.defines)
            .and_then(|d| d.get(node_text))
            .or_else(|| {
                program = None; // found global, clear program
                defs.global_defines.get(node_text)
            })?;

        log::debug!("found define");
        Some(SymbolDefinition {
            statement_range: ts_range_to_lsp(pd.statement_range),
            program,
            typ: SymbolType::Define {
                value: &doc_text[pd.value_range.start_byte..pd.value_range.end_byte],
                named: NamedSymbol {
                    text: node_text,
                    name_range: ts_range_to_lsp(pd.name_range),
                },
            },
        })
    }

    // find all references to a symbol. return the name of the definition (as range) and all references
    pub fn get_references<'a>(&'a self, node: Node) -> Option<(Range, Cow<'a, [Range]>)> {
        static DEFINE_REFERENCE: LazyLock<tree_sitter::Query> = LazyLock::new(|| {
            pio_query(
                r#"
(directive
    (directive_program
        program_name: _                         @program_name
    )
)

(symbol)                                        @sym
"#,
            )
        });

        log::info!("searching for references to {:?}", node);

        let definition = self.get_definition(node)?;
        log::info!("found definition: {:?}", definition);

        let doc_text = self.programs.backing_cart();

        let (name_range, references) = match &definition.typ {
            SymbolType::Define { named, value: _ } => {
                // compute defines only when needed (TODO: cache)
                let mut cursor = QueryCursor::new();
                let mut it = cursor.matches(
                    &DEFINE_REFERENCE,
                    self.tree.root_node(),
                    doc_text.as_bytes(),
                );

                // find all matching symbols
                let mut refs = Vec::new();
                let mut program = None;
                while let Some(m) = it.next() {
                    let n = m.captures()[0].node;
                    match m.pattern_index {
                        0 if definition.program.is_some() => {
                            program = Some(&doc_text[n.byte_range()])
                        }
                        1 if program == definition.program
                            && Some(&doc_text[n.byte_range()]) == definition.text() =>
                        {
                            refs.push(ts_range_to_lsp(n.range()))
                        }
                        _ => {}
                    }
                }
                (named.name_range, Cow::Owned(refs))
            }
            SymbolType::Label(named) => (
                named.name_range,
                Cow::Borrowed(
                    definition
                        .program
                        .map(|p| &self.programs.get().programs[p].labels[named.text].references[..])
                        .unwrap_or_default(),
                ),
            ),
            SymbolType::Wrap => {
                let p = definition
                    .program
                    .map(|p| &self.programs.get().programs[p])?;
                (
                    p.wrap_target?,
                    Cow::Borrowed(std::slice::from_ref(p.wrap.as_ref()?)),
                )
            }
        };

        Some((name_range, references))
    }

    /// look for the things the lsp in interested like "jump to definition" and inlay hints and populate self
    pub fn analyze_programs<'a>(
        &mut self,
        text: &'a str,
        size_hint: (usize, usize),
    ) -> Programs<'a> {
        // search for .program statements or instructions
        static PROG_INSTR_QUERY: LazyLock<tree_sitter::Query> = LazyLock::new(|| {
            pio_query(
                r#"
(directive
    (directive_program
        program_name: _                         @program_name
    )
)                                               @program

(instruction)                                   @instr

(label
    label: (symbol)                             @label_name
)                                               @label_statement

(define
    define_symbol: (symbol)                     @define_name
    define_value: _                             @define_value
)                                               @define_statement

(label_reference (value)                        @label_ref
)

(directive (directive_wrap))                    @wrap
(directive (directive_wrap_target))             @wrap_target
"#,
            )
        });

        let mut programs = HashMap::with_capacity(size_hint.0);
        let mut global_defines = HashMap::with_capacity(size_hint.1);
        self.inlay_hints.clear();

        let mut program = None; // current program
        let mut labels_for_instr = Vec::new(); // labels for the current instruction (could have multiple in a row for the same instruction)
        let mut label_refs = Vec::new(); // all references to labels we find
        let mut pc = 0;
        let mut it = self
            .cursor
            .matches(&PROG_INSTR_QUERY, self.tree.root_node(), text.as_bytes());
        while let Some(m) = it.next() {
            match m.pattern_index {
                /*program directive*/
                0 => {
                    let statement = m.captures()[0];
                    debug_assert_eq!(
                        PROG_INSTR_QUERY.capture_names()[statement.index as usize],
                        "program"
                    );

                    let name = m.captures()[1];
                    debug_assert_eq!(
                        PROG_INSTR_QUERY.capture_names()[name.index as usize],
                        "program_name"
                    );

                    let name = &text[name.node.byte_range()];
                    let r = ts_range_to_lsp(statement.node.range());
                    if let Some((name, v)) = program.replace((name, ProgramInfo::new(r))) {
                        self.inlay_hints.push(v.prog_hint());
                        programs.insert(name, v);
                    }
                    pc = 0;
                }
                /*instruction*/
                1 => {
                    if let Some((_, v)) = program.as_mut() {
                        v.instr_count += 1;

                        // check if label(s) before instruction
                        for (name, statement_range, name_range) in labels_for_instr.drain(..) {
                            v.labels.insert(
                                name,
                                ProgramLabel {
                                    statement_range,
                                    name_range,
                                    instr_pc: pc,
                                    references: Vec::new(),
                                },
                            );

                            // hint with pc for label
                            // self.inlay_hints.push(InlayHint {
                            //     position: tree_sitter_to_lsp_range(statement).end,
                            //     label: Label::String(format!("{pc:02}")),
                            //     kind: Some(InlayHintKind::Type),
                            //     padding_left: Some(true),
                            //     padding_right: Some(false),
                            //     text_edits: None,
                            //     tooltip: None,
                            //     data: None,
                            // });
                        }

                        // hint with pc for instruction
                        let position = ts_range_to_lsp(m.captures()[0].node.range()).start;
                        self.inlay_hints.push(InlayHint {
                            position,
                            label: Label::String(format!("{pc:02}:")),
                            kind: Some(InlayHintKind::Type),
                            padding_left: Some(false),
                            padding_right: Some(true),
                            text_edits: None,
                            tooltip: None,
                            data: None,
                        });
                        pc += 1;
                    }
                }
                /*label*/
                2 => {
                    let statement = m.captures()[0];
                    debug_assert_eq!(
                        PROG_INSTR_QUERY.capture_names()[statement.index as usize],
                        "label_statement"
                    );

                    let name = m.captures()[1];
                    debug_assert_eq!(
                        PROG_INSTR_QUERY.capture_names()[name.index as usize],
                        "label_name"
                    );

                    labels_for_instr.push((
                        &text[name.node.byte_range()],
                        statement.node.range(),
                        name.node.range(),
                    ));
                }
                /*define*/
                3 => {
                    let statement = m.captures()[0];
                    debug_assert_eq!(
                        PROG_INSTR_QUERY.capture_names()[statement.index as usize],
                        "define_statement"
                    );

                    let name = m.captures()[1];
                    debug_assert_eq!(
                        PROG_INSTR_QUERY.capture_names()[name.index as usize],
                        "define_name"
                    );

                    let value = m.captures()[2];
                    debug_assert_eq!(
                        PROG_INSTR_QUERY.capture_names()[value.index as usize],
                        "define_value"
                    );

                    let defines = program
                        .as_mut()
                        .map(|(_, v)| &mut v.defines)
                        .unwrap_or(&mut global_defines);

                    defines.insert(
                        &text[name.node.byte_range()],
                        ProgramDefine {
                            statement_range: statement.node.range(),
                            value_range: value.node.range(),
                            name_range: name.node.range(),
                        },
                    );
                }
                /*label ref*/
                4 => {
                    if let Some((p, _)) = program.as_ref() {
                        label_refs.push((m.captures()[0].node, *p));
                    }
                }
                /*.wrap*/
                /*.wrap_target*/
                5 | 6 => {
                    if let Some((_, p)) = program.as_mut() {
                        *(if m.pattern_index == 5 {
                            &mut p.wrap
                        } else {
                            &mut p.wrap_target
                        }) = Some(ts_range_to_lsp(m.captures()[0].node.range()));
                    }
                }

                _ => unreachable!(),
            }
        }
        if let Some((name, v)) = program {
            self.inlay_hints.push(v.prog_hint());
            programs.insert(name, v);
        }

        // insert label references into ProgramInfo and generate inlay hint
        // TODO: do this in single pass
        for (n, program) in label_refs {
            let text = &text[n.byte_range()];
            if let Some(l) = programs
                .get_mut(&program)
                .and_then(|v| v.labels.get_mut(text))
            {
                self.inlay_hints.push(InlayHint {
                    position: ts_range_to_lsp(n.range()).end,
                    label: Label::String(format!("{:02}", l.instr_pc)),
                    kind: Some(InlayHintKind::Type),
                    padding_left: Some(true),
                    padding_right: Some(true),
                    text_edits: None,
                    tooltip: None,
                    data: None,
                });
                l.references.push(ts_range_to_lsp(n.range()));
            }
        }

        // free up memory if the number of inlay hints shrinks
        self.inlay_hints.shrink_to_fit();

        Programs {
            programs,
            global_defines,
        }
    }

    // TODO: delay / sideset, directives
    /// generate hover documentation for instructions
    fn hover_instr(&self, mut node: Node<'_>) -> Option<String> {
        use crate::doc::*;

        let doc_text = self.programs.backing_cart();

        // check if hovering op keyword (jmp, wait, ...) or in an unspecified part of the instruction
        let kind = node.kind();
        if kind == "opcode" || kind.starts_with("instr_") {
            // opcode text
            let op_node = if kind == "opcode" {
                node
            } else {
                // find the opcode child
                let mut cursor = node.walk();
                node.children(&mut cursor).find(|e| e.kind() == "opcode")?
            };
            let opcode = doc_text[op_node.byte_range()].to_uppercase();

            // concatenate docs of all MOV variants
            if opcode == "MOV" {
                let mov_it = INSTRUCTION_DOC
                    .into_iter()
                    .filter(|(k, _)| k.starts_with("MOV"));
                let len = mov_it.clone().map(|(k, v)| 4 + k.len() + v.len()).sum();
                let mut cont = String::with_capacity(len);
                for (k, d) in mov_it {
                    writeln!(&mut cont, "# {k}\n{d}").unwrap();
                }
                debug_assert_eq!(len, cont.len());
                return Some(cont);
            }

            // return doc for instruction
            if let Some((_, d)) = INSTRUCTION_DOC.into_iter().find(|e| e.0 == opcode) {
                return Some(d.to_string());
            }
        }

        // check if inside operant
        let instr_path = get_instruction(node)?;
        log::debug!("looking for operand for {}", node.kind());
        log::debug!(
            "instr_path is {:?}",
            instr_path.iter().map(Node::kind).collect::<Vec<_>>(),
        );

        // find opcode node
        let opcode = instr_path
            .last()
            .unwrap()
            .child_by_field_name("op")
            .and_then(|op| {
                let mut cursor = op.walk();
                op.children(&mut cursor).find(|c| c.kind() == "opcode")
            })?;

        // opcode text
        let opcode = doc_text[opcode.byte_range()].to_uppercase();
        log::debug!("opcode is {opcode}");

        // try find operand dependent of the instruction node (this moves up the tree to the operand position in case we are in an expression)
        if let Some(op_child_operand) = instr_path.len().checked_sub(3).map(|i| instr_path[i])
            /* because of wait irq next/prev */
            && kind != "irq_target"
        {
            node = op_child_operand;
        }

        // find Assembler Syntax table for instruction and try match with node
        let (_, d) = INSTRUCTIONS.into_iter().find(|(k, _)| k == &opcode)?;
        log::debug!("found asm table for {opcode}");

        // literal text (e.g., block, noblock)
        let node_text = doc_text[node.byte_range()].to_lowercase();
        if let Some((_, literal)) = d.iter().find(|(k, _)| k == &node_text) {
            log::debug!("found literal {literal}");
            return Some(literal.to_string());
        }

        // handle "irq set", "irq wait", ...
        if opcode == "IRQ"
            && let Some((_, literal)) = d
                .iter()
                .find(|(k, _)| k.split_once(' ').map(|e| e.1) == Some(&node_text))
        {
            log::debug!("found irq literal {literal}");
            return Some(literal.to_string());
        }

        // check if we are in target of jmp instruction
        if opcode == "JMP" && is_label(node) {
            let (_, operand_desc) = d.iter().find(|(k, _)| *k == "<target>").unwrap();
            log::debug!("found jmp operand_desc {operand_desc}");
            return Some(operand_desc.to_string());
        }

        // search for matching entry in Assembler Syntax table
        if let Some(mut operand) = get_named_node_field_name(node) {
            log::debug!("found operand {operand} for {node:?}");

            // handle wait special cases
            if opcode == "WAIT" && operand == "source" {
                if instr_path[0].kind() == "src" {
                    return Some("gpio | pin | irq ( prev | next ) | jmppin".to_string());
                }

                for e in ["gpio_num", "pin_num", "irq_num"] {
                    if node.child_by_field_name(e).is_some() {
                        operand = e;
                    }
                }
            }

            if let Some((_, operand_desc)) = d.iter().find(|(k, _)| {
                let k = k.split_once(' ').map(|e| e.0).unwrap_or(k); // for irq (rel)
                k.trim_start_matches('<').trim_end_matches('>') == operand
            }) {
                log::debug!("found operand_desc {operand_desc} for operand {operand}");
                return Some(operand_desc.to_string());
            } else {
                log::warn!("no description for operand {operand}");
            }
        } else {
            log::debug!("no operand found");
        }

        None
    }
}

pub fn lsp(pioasm: Option<PathBuf>) -> std::result::Result<(), Box<dyn Error + Sync + Send>> {
    log::info!("starting piolsp {}", env!("CARGO_PKG_VERSION"));

    // transport
    let (connection, io_thread) = Connection::stdio();

    let (id, init_params) = connection.initialize_start()?;
    let init: InitializeParams = serde_json::from_value(init_params)?;
    assert!(
        init.capabilities
            .general
            .as_ref()
            .unwrap()
            .position_encodings
            .as_ref()
            .unwrap()
            .contains(&PositionEncodingKind::UTF8),
        "client does not support UTF-8 encoding (piolsp is not quite following spec here because it does not support UTF-16 yet)"
    );
    // log::debug!("init: {init:#?}");

    let td_caps = init.capabilities.text_document.as_ref();

    // advertised capabilities
    let caps = ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF8),
        text_document_sync: Some(TextDocumentSync::Kind(TextDocumentSyncKind::Incremental)),

        definition_provider: Some(DefinitionProvider::Bool(true)),
        references_provider: Some(ReferencesProvider::Bool(true)),
        hover_provider: Some(HoverProvider::Bool(true)),
        document_formatting_provider: Some(DocumentFormattingProvider::Bool(true)),
        inlay_hint_provider: Some(InlayHintProvider::Bool(true)),
        rename_provider: Some(RenameProvider::RenameOptions(RenameOptions {
            prepare_provider: td_caps
                .and_then(|e| e.rename.as_ref())
                .and_then(|e| e.prepare_support),
            ..Default::default()
        })),
        document_highlight_provider: Some(DocumentHighlightProvider::Bool(true)),

        // completion_provider: Some(CompletionOptions::default()),
        ..Default::default()
    };

    connection.initialize_finish(id, serde_json::json!({"capabilities": caps}))?;
    main_loop(connection, pioasm.as_deref())?;
    io_thread.join()?;

    log::info!("shutting down piolsp");
    Ok(())
}

fn main_loop(
    connection: Connection,
    pioasm: Option<&Path>,
) -> std::result::Result<(), Box<dyn Error + Sync + Send>> {
    let mut docs = HashMap::default();
    let mut parser = pio_parser();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
                if let Err(err) = handle_request(&connection, &req, &docs) {
                    log::error!("[lsp] request {} failed: {err}", req.method);
                }
            }
            Message::Notification(note) => {
                if let Err(err) =
                    handle_notification(&connection, &note, &mut docs, pioasm, &mut parser)
                {
                    log::error!("[lsp] notification {} failed: {err}", note.method);
                }
            }
            Message::Response(resp) => log::error!("[lsp] response: {resp:?}"),
        }
    }
    Ok(())
}

fn range_to_offset(document: &str, range: Range) -> std::ops::Range<usize> {
    log::info!("r = {range:?}");
    log::info!("doc = {document:?}");

    let mut line_start = 0;
    let mut byte_range = [(range.start, !0), (range.end, !0)];

    // need to include '\n'
    let mut line_count = 0;
    for line_text in document.split_inclusive('\n') {
        for (pos, found) in &mut byte_range {
            if line_count == pos.line as usize {
                debug_assert!(line_text.len() >= pos.character as usize,);
                #[cfg(debug_assertions)]
                if !line_text.is_char_boundary(pos.character as _) {
                    log::error!("{}:{} is not a UTF-8 boundary", pos.line, pos.character);
                }
                *found = line_start + pos.character as usize;
            }
        }
        let (start, end) = (byte_range[0].1, byte_range[1].1);
        if start != !0 && end != !0 {
            return start..end;
        }

        line_start += line_text.len();
        line_count += 1;
    }

    // check if range is last \n
    for (pos, found) in &mut byte_range {
        if line_count == pos.line as usize && pos.character == 0 {
            *found = line_start;
        }
    }
    let (start, end) = (byte_range[0].1, byte_range[1].1);
    if start != !0 && end != !0 {
        return start..end;
    }

    // this should never happen if the client follows spec
    log::error!("range is past end of document {line_count } {byte_range:?}");

    // something went wrong, clamp to document size
    start.min(document.len())..end.min(document.len())
}

fn point_after_insert(start: tree_sitter::Point, inserted: &str) -> tree_sitter::Point {
    let newline_count = memchr::memchr_iter(b'\n', inserted.as_bytes()).count();
    if newline_count == 0 {
        tree_sitter::Point {
            row: start.row,
            column: start.column + inserted.len(),
        }
    } else {
        let last_line = inserted.rsplit('\n').next().unwrap();
        tree_sitter::Point {
            row: start.row + newline_count,
            column: last_line.len(),
        }
    }
}

fn apply_doc_changes(
    doc: &mut String,
    mut changes: Vec<TextDocumentContentChangeEvent>,
) -> Option<Vec<tree_sitter::InputEdit>> {
    if let [
        TextDocumentContentChangeEvent::TextDocumentContentChangeWholeDocument(
            TextDocumentContentChangeWholeDocument { text },
        ),
    ] = changes.as_mut_slice()
    {
        *doc = std::mem::take(text);
        return None;
    };

    log::warn!("{:?}", changes);
    let mut tree_edits = Some(Vec::new());
    for change in changes {
        match change {
            TextDocumentContentChangeEvent::TextDocumentContentChangePartial(
                TextDocumentContentChangePartial { range, text, .. },
            ) => {
                let r = range_to_offset(doc, range);
                doc.replace_range(r.clone(), &text);
                if let Some(tree_edits) = tree_edits.as_mut() {
                    tree_edits.push(tree_sitter::InputEdit {
                        start_byte: r.start,
                        old_end_byte: r.end,
                        new_end_byte: r.start + text.len(),
                        start_position: lsp_pos_to_ts(range.start),
                        old_end_position: lsp_pos_to_ts(range.end),
                        new_end_position: point_after_insert(lsp_pos_to_ts(range.start), &text),
                    });
                }
            }
            TextDocumentContentChangeEvent::TextDocumentContentChangeWholeDocument(
                TextDocumentContentChangeWholeDocument { text },
            ) => {
                log::warn!("unexpected TextDocumentContentChangeWholeDocument");
                *doc = text;
                tree_edits = None;
            }
        }
    }
    tree_edits
}

fn parse_node_params<T: Notification>(
    note: &lsp_server::Notification,
) -> Result<T::Params, serde_json::Error> {
    serde_json::from_value(note.params.clone())
}
fn handle_notification(
    conn: &Connection,
    note: &lsp_server::Notification,
    docs: &mut HashMap<Uri, DocumentData>,
    pioasm: Option<&Path>,
    parser: &mut tree_sitter::Parser,
) -> Result<(), Box<dyn std::error::Error>> {
    let method = note.method.as_str().into();
    match method {
        DidOpenTextDocumentNotification::METHOD => {
            // opened new file, parse with tree sitter and check with pioasm

            let p = parse_node_params::<DidOpenTextDocumentNotification>(note)?;
            let uri = p.text_document.uri;

            let mut doc = docs
                .entry(uri.clone())
                .insert_entry(DocumentData::new(parser, p.text_document.text));
            check_file_pioasm(conn, uri, doc.get_mut(), pioasm)?;
        }
        DidSaveTextDocumentNotification::METHOD => {
            // saved file, check with pioasm

            let p = parse_node_params::<DidSaveTextDocumentNotification>(note)?;
            log::info!("{:?}", p.text);
            let uri = p.text_document.uri;
            let doc = docs
                .get_mut(&uri)
                .ok_or_else(|| format!("no doc for {uri}"))?;
            if let Some(t) = p.text {
                assert_eq!(t.as_str(), doc.programs.backing_cart().as_str());
            }
            check_file_pioasm(conn, uri, doc, pioasm)?;
        }
        DidCloseTextDocumentNotification::METHOD => {
            // file was closed, remove from docs

            let p = parse_node_params::<DidCloseTextDocumentNotification>(note)?;
            docs.remove(&p.text_document.uri);
        }
        DidChangeTextDocumentNotification::METHOD => {
            // source was changed, reparse tree

            let p = parse_node_params::<DidChangeTextDocumentNotification>(note)?;
            let uri = p.text_document.text_document_identifier.uri;

            let doc = docs
                .get_mut(&uri)
                .ok_or_else(|| format!("no doc for {uri}"))?;

            let mut text = std::mem::replace(&mut doc.programs, DocumentData::default_yoke())
                .into_backing_cart();

            let tree_edits = apply_doc_changes(&mut text, p.content_changes);
            let old_tree = tree_edits.map(|edits| {
                let mut tree = doc.tree.clone(); // TODO
                for edit in edits {
                    tree.edit(&edit);
                }
                tree
            });
            doc.update(parser, text, old_tree.as_ref());
        }
        e => log::debug!("ignoring notification {e:?}"),
    }
    Ok(())
}

fn process_request_td<T: Request>(
    conn: &Connection,
    req: &ServerRequest,
    docs: &HashMap<Uri, DocumentData>,
    get_td: impl FnOnce(&T::Params) -> &TextDocumentIdentifier,
    process: impl FnOnce(&DocumentData) -> T::Result,
) -> Result<(), Box<dyn std::error::Error>> {
    let params: T::Params = serde_json::from_value(req.params.clone())?;
    let td = get_td(&params);
    let uri = &td.uri;
    let doc = docs.get(uri).ok_or_else(|| format!("no doc for {uri}"))?;
    let result = process(doc);
    send_ok(conn, req.id.clone(), &result)
}
fn process_request<T: Request>(
    conn: &Connection,
    req: &ServerRequest,
    docs: &HashMap<Uri, DocumentData>,
    get_tdp: impl FnOnce(&T::Params) -> &TextDocumentPositionParams,
    process: impl FnOnce(&T::Params, &DocumentData, &Uri, Position) -> T::Result,
) -> Result<(), Box<dyn std::error::Error>> {
    let params: T::Params = serde_json::from_value(req.params.clone())?;
    let tdp = get_tdp(&params);
    let uri = &tdp.text_document.uri;
    let doc = docs.get(uri).ok_or_else(|| format!("no doc for {uri}"))?;
    let result = process(&params, doc, uri, tdp.position);
    send_ok(conn, req.id.clone(), &result)
}

fn handle_definition_req(
    _: &DefinitionParams,
    doc: &DocumentData,
    uri: &Uri,
    pos: Position,
) -> Option<DefinitionResponse> {
    doc.node_at(pos)
        .and_then(|sym| doc.get_definition(sym))
        .map(|d| {
            DefinitionResponse::Definition(Definition::Location(Location {
                uri: uri.clone(),
                range: d.statement_range,
            }))
        })
}
fn handle_references_req(
    params: &ReferenceParams,
    doc: &DocumentData,
    uri: &Uri,
    pos: Position,
) -> Option<Vec<Location>> {
    doc.node_at(pos)
        .and_then(|n| doc.get_references(n))
        .map(|(def, refs)| {
            params
                .context
                .include_declaration
                .then_some(def)
                .into_iter()
                .chain(refs.iter().copied())
                .map(|range| Location {
                    uri: uri.clone(),
                    range,
                })
                .collect()
        })
}
fn handle_prepare_rename_req(
    _: &PrepareRenameParams,
    doc: &DocumentData,
    _: &Uri,
    pos: Position,
) -> Option<PrepareRenameResult> {
    doc.node_at(pos).and_then(|sym| {
        Some(PrepareRenameResult::Range(
            doc.get_definition(sym).and_then(|d| d.name_range())?,
        ))
    })
}
fn handle_rename_req(
    params: &RenameParams,
    doc: &DocumentData,
    uri: &Uri,
    pos: Position,
) -> Option<WorkspaceEdit> {
    doc.node_at(pos)
        .and_then(|n| doc.get_references(n))
        .map(|(def, refs)| {
            let edits = std::iter::once(def)
                .chain(refs.iter().copied())
                .map(|range| TextEdit {
                    range,
                    new_text: params.new_name.clone(),
                })
                .collect();
            WorkspaceEdit {
                changes: Some(std::iter::once((uri.clone(), edits)).collect()),
                document_changes: None,
                change_annotations: None,
            }
        })
}
fn handle_hover_req(_: &HoverParams, doc: &DocumentData, _: &Uri, pos: Position) -> Option<Hover> {
    let node = doc.node_at(pos)?;

    // hovering instruction
    if let Some(value) = doc.hover_instr(node) {
        return Some(Hover {
            contents: Contents::MarkupContent(MarkupContent {
                value,
                kind: MarkupKind::Markdown,
            }),
            range: None,
        });
    }

    // TODO: check for directives here

    // check if we are hovering a definition
    if let Some(SymbolDefinition {
        typ: SymbolType::Define { value, .. },
        ..
    }) = doc.get_definition(node)
    {
        return Some(Hover {
            contents: Contents::MarkupContent(MarkupContent {
                value: value.into(),
                kind: MarkupKind::PlainText,
            }),
            range: None,
        });
    }

    None
}
fn handle_highlight_req(
    _: &DocumentHighlightParams,
    doc: &DocumentData,
    _: &Uri,
    pos: Position,
) -> Option<Vec<DocumentHighlight>> {
    doc.node_at(pos)
        .and_then(|n| doc.get_references(n))
        .map(|(def, refs)| {
            std::iter::once(DocumentHighlight::new(
                def,
                Some(DocumentHighlightKind::Write),
            ))
            .chain(
                refs.iter()
                    .map(|&range| DocumentHighlight::new(range, Some(DocumentHighlightKind::Read))),
            )
            .collect()
        })
}

fn handle_request(
    conn: &Connection,
    req: &ServerRequest,
    docs: &HashMap<Uri, DocumentData>,
) -> Result<(), Box<dyn std::error::Error>> {
    let parsed: LspRequestMethod<'_> = req.method.as_str().into();
    match parsed {
        DefinitionRequest::METHOD => process_request::<DefinitionRequest>(
            conn,
            req,
            docs,
            |p| &p.text_document_position_params,
            handle_definition_req,
        ),
        ReferencesRequest::METHOD => process_request::<ReferencesRequest>(
            conn,
            req,
            docs,
            |p| &p.text_document_position_params,
            handle_references_req,
        ),
        PrepareRenameRequest::METHOD => process_request::<PrepareRenameRequest>(
            conn,
            req,
            docs,
            |p| &p.text_document_position_params,
            handle_prepare_rename_req,
        ),
        RenameRequest::METHOD => process_request::<RenameRequest>(
            conn,
            req,
            docs,
            |p| &p.text_document_position_params,
            handle_rename_req,
        ),
        HoverRequest::METHOD => process_request::<HoverRequest>(
            conn,
            req,
            docs,
            |p| &p.text_document_position_params,
            handle_hover_req,
        ),
        DocumentHighlightRequest::METHOD => process_request::<DocumentHighlightRequest>(
            conn,
            req,
            docs,
            |p| &p.text_document_position_params,
            handle_highlight_req,
        ),

        DocumentFormattingRequest::METHOD => process_request_td::<DocumentFormattingRequest>(
            conn,
            req,
            docs,
            |p| &p.text_document,
            |doc| {
                let doc_text = doc.programs.backing_cart();
                Some(vec![TextEdit {
                    range: full_range(doc_text),
                    new_text: format_tree(doc_text, doc.tree.root_node()),
                }])
            },
        ),
        InlayHintRequest::METHOD => process_request_td::<InlayHintRequest>(
            conn,
            req,
            docs,
            |p| &p.text_document,
            |doc| Some(doc.inlay_hints.clone()),
        ),
        _ => send_err(
            conn,
            req.id.clone(),
            lsp_server::ErrorCode::MethodNotFound,
            "unhandled method",
        ),
    }
}

/// check if a file compiles with pioasm and send diagnostics if it doesn't
fn check_file_pioasm(
    conn: &Connection,
    uri: Uri,
    doc: &mut DocumentData,
    pioasm: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    // parse pioasm diagnostics
    static DIAGNOSTICS_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new( r"(?m)^:(?<row>\d+).(?<col_start>\d+)(?:-(?<col_end>\d+))?:\s*(?<msg>.*)\n\s*(?<row2>\d+)\s*\|(?<orig_line>.*)\n\s*\|(?<pos_ind>\s*\^~*)\s*$|^\n*too many errors; aborting\.\n*$").unwrap()
    });

    let mk_diag = |message: gen_lsp_types::Message, range: Option<Range>| Diagnostic {
        severity: Some(DiagnosticSeverity::Error),
        source: Some("pioasm".into()),
        message,
        range: range.unwrap_or_default(),
        ..Default::default()
    };

    match run_pioasm(doc.programs.backing_cart(), pioasm) {
        Ok(out) => {
            // pioasm does not support non ASCII input and may output "invalid character: " of the individual UTF-8 bytes which may not be valid ASCII
            // so the output is not be valid UTF-8 then
            // need to take some care to still return valid byte indices in that case
            let stderr = String::from_utf8_lossy(&out.stderr);

            let mut diagnostics = Vec::new();
            let mut ind = 0;
            for c in DIAGNOSTICS_RE.captures_iter(&stderr) {
                // make sure we can parse the entire output
                let start = c.get_match().start();
                let end = c.get_match().end();
                if start > ind {
                    log::error!("couldn't parse {}..{}: {}", ind, start, &stderr[ind..start]);
                }
                ind = end + 1;

                // check for "too many errors" (can only happen if there is no row field)
                let Some(row) = c.name("row") else {
                    if !c.get_match().as_str().contains("too many errors") {
                        return Err("no row".into());
                    }
                    diagnostics.push(mk_diag("too many errors; aborting.".into(), None));
                    continue; // there should not be any output after this, continue to make sure
                };

                // parse captures (pioasm starts line/col at 1)
                let row = row
                    .as_str()
                    .parse::<u32>()
                    .map_err(|err| format!("failed to parse row: {err}"))?
                    .saturating_sub(1);
                let col_start = c
                    .name("col_start")
                    .ok_or("no col start")?
                    .as_str()
                    .parse::<u32>()
                    .map_err(|err| format!("failed to parse col_start: {err}"))?
                    .saturating_sub(1);
                let col_start = stderr.floor_char_boundary(col_start as _) as _; // fix byte index in case pioasm encounters non ASCII input

                let col_end = c
                    .name("col_end")
                    .map(|c| {
                        c.as_str()
                            .parse::<u32>()
                            .map_err(|err| format!("failed to parse col_end: {err}"))
                            .map(|col_end| stderr.floor_char_boundary(col_end as _) as _) // see comment above
                    })
                    .unwrap_or(Ok(col_start))?;

                let msg = c.name("msg").ok_or("no msg")?.as_str();

                diagnostics.push(mk_diag(
                    msg.into(),
                    Some(Range::new(
                        Position::new(row, col_start),
                        Position::new(row, col_end),
                    )),
                ));
            }

            // ensure no remaining output
            if ind < stderr.len() {
                log::error!("couldn't parse {}..: {}", ind, &stderr[ind..]);
            }

            // send result
            let params: <PublishDiagnosticsNotification as Notification>::Params =
                PublishDiagnosticsParams {
                    uri: uri.clone(),
                    diagnostics,
                    version: None,
                };
            conn.sender
                .send(Message::Notification(lsp_server::Notification::new(
                    PublishDiagnosticsNotification::METHOD.into(),
                    params,
                )))?;

            if !out.stdout.is_empty() {
                log::debug!("pioasm: {}", str::from_utf8(&out.stdout)?);

                // store output
                // match serde_json::from_str::<Value>(str::from_utf8(&out.stdout)?) {
                //     Ok(root) => {
                //         let programs = root.get("programs").and_then(|value| value.as_array()).map(
                //             |programs| {
                //                 programs
                //                     .iter()
                //                     .filter_map(|p| {
                //                         Some((
                //                             p.get("name")
                //                                 .and_then(Value::as_str)
                //                                 .map(str::to_owned)?,
                //                             p.get("instructions")
                //                                 .and_then(Value::as_array)
                //                                 .cloned()?,
                //                         ))
                //                     })
                //                     .collect::<BTreeMap<_, _>>()
                //             },
                //         );
                //         if let Some(programs) = programs {
                //             doc.programs = Some(programs);
                //             // conn.sender.send(Message::Request(lsp_server::Request {
                //             //     id: lsp_server::RequestId::from("inlay-refresh".to_owned()),
                //             //     method: "workspace/inlayHint/refresh".into(),
                //             //     params: serde_json::Value::Null,
                //             // }))?;
                //         } else {
                //             log::warn!("failed to parse pioasm output");
                //         }
                //     }
                //     Err(err) => log::warn!("pioasm output is not valid JSON: {err}"),
                // }
            }
        }
        Err(err) => log::warn!("failed to run pioasm: {err}"),
    }

    Ok(())
}

/// full range of the text
fn full_range(text: &str) -> Range {
    let last_line_idx = text.lines().count().saturating_sub(1) as u32;
    let last_col = text.lines().next_back().map_or(0, str::len) as u32;
    Range::new(Position::new(0, 0), Position::new(last_line_idx, last_col))
}

fn send_ok<T: serde::Serialize>(
    conn: &Connection,
    id: RequestId,
    result: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let resp = Response {
        id,
        response_result: Ok(serde_json::to_value(result)?),
    };
    conn.sender.send(Message::Response(resp))?;
    Ok(())
}

fn send_err(
    conn: &Connection,
    id: RequestId,
    code: lsp_server::ErrorCode,
    msg: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    log::error!("err for {id}: {msg}");
    let resp = Response {
        id,
        response_result: Err(lsp_server::ResponseError {
            code: code as i32,
            message: msg.into(),
            data: None,
        }),
    };
    conn.sender.send(Message::Response(resp))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn doc(src: &str) -> DocumentData {
        // static LOGGER_INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        // let _ = LOGGER_INIT.get_or_init(|| simple_logging::log_to_stderr(log::LevelFilter::Info));

        DocumentData::new(&mut pio_parser(), src.to_string())
    }

    #[test]
    fn test_node_at() {
        let src = r#"
.program p
jmp 0
"#;

        let doc = doc(src);

        let node = doc.node_at(Position::new(2, 0)).unwrap();
        assert_eq!(node.kind(), "opcode");

        let node = doc.node_at(Position::new(2, 1)).unwrap();
        assert_eq!(node.kind(), "opcode");

        let node = doc.node_at(Position::new(2, 2)).unwrap();
        assert_eq!(node.kind(), "opcode");

        let node = doc.node_at(Position::new(2, 3)).unwrap();
        assert_eq!(node.kind(), "instr_jmp");

        let node = doc.node_at(Position::new(2, 4)).unwrap();
        assert_eq!(node.kind(), "integer");

        let node = doc.node_at(Position::new(2, 5)).unwrap();
        assert_eq!(node.kind(), "\n");
    }

    #[test]
    fn count_instructions() {
        let src = r#"
.program test
    set pins, 1
    nop
    jmp start

start:
    nop
"#;
        let doc = doc(src);

        let prog = doc.programs.get().programs.get("test").unwrap();
        assert_eq!(prog.instr_count, 4);
    }

    #[test]
    fn finds_global_define() {
        let src = r#"
.define FOO 2
.program test
    set x, FOO
"#;
        let doc = doc(src);

        let pos = Position::new(3, 13); // somewhere on FOO
        let node = doc.node_at(pos).unwrap();
        let def = doc.get_definition(node).unwrap();
        assert_eq!(def.text(), Some("FOO"));

        match def.typ {
            SymbolType::Define { value, named: _ } => assert_eq!(value, "2"),
            _ => panic!("expected define"),
        }
    }

    #[test]
    fn finds_label_definition() {
        let src = r#"
.program p
start:
    nop
    jmp start
"#;

        let doc = doc(src);

        let pos = Position::new(4, 10); // start
        let node = doc.node_at(pos).unwrap();

        let def = doc.get_definition(node).unwrap();

        match def.typ {
            SymbolType::Label(_) => {}
            _ => panic!("expected label"),
        }

        assert_eq!(def.text().unwrap(), "start");
    }

    #[test]
    fn collects_all_label_references() {
        let doc = doc(r#"
.program p
start:
    nop
    jmp start
    jmp start
"#);

        let pos = Position::new(2, 1);
        let node = doc.node_at(pos).unwrap();

        let (_, refs) = doc.get_references(node).unwrap();

        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn creates_instruction_pc_hints() {
        let doc = doc(r#"
.program p
    nop
    nop
    nop
"#);

        let labels: Vec<_> = doc
            .inlay_hints
            .iter()
            .filter_map(|h| match &h.label {
                Label::String(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();

        assert!(labels.contains(&"00:"));
        assert!(labels.contains(&"01:"));
        assert!(labels.contains(&"02:"));
        assert!(labels.iter().any(|s| s.contains("instructions")));
    }

    #[test]
    fn formatting_is_idempotent() {
        let d = doc(r#"
.program p
    nop
"#);

        let formatted = format_tree(d.programs.backing_cart(), d.tree.root_node());

        let doc2 = doc(&formatted);
        let formatted2 = format_tree(doc2.programs.backing_cart(), doc2.tree.root_node());

        assert_eq!(formatted, formatted2);
    }

    #[test]
    fn formatting_basic() {
        let doc = doc(r#"
.program p


nop
set x 23
"#);

        let expected = r#"
.program p

    nop
    set x, 23
"#;

        let formatted = format_tree(doc.programs.backing_cart(), doc.tree.root_node());
        assert_eq!(formatted, expected);
    }

    #[test]
    fn computes_full_range() {
        let text = "abc\ndef";
        let r = full_range(text);
        assert_eq!(r.start, Position::new(0, 0));
        assert_eq!(r.end, Position::new(1, 3));
    }

    //     #[test]
    //     fn completion() {
    //         let doc = doc(r#".program p
    // mov
    // "#);

    //         dbg!(doc.tree.root_node().to_sexp());

    //         let c = doc
    //             .completion_items(doc.node_at(Position::new(1, 2)).unwrap())
    //             .unwrap();

    //         // dbg!(&c);

    //         assert!(c.is_empty());
    //     }

    fn test_hover(src: &'static str, c: u32, expected: impl Into<Option<&'static str>>) {
        let doc = doc(&format!(".program p\n{src}"));

        let c = doc
            .hover_instr(doc.node_at(Position::new(1, c)).unwrap())
            .unwrap_or_default();

        if let Some(expected) = expected.into() {
            if c.len() < expected.len() {
                panic!("output too short: {c}\nexpected:         {expected}");
            }
            assert_eq!(&c[..expected.len()], expected);
        } else {
            assert_eq!(c, "");
        }
    }

    #[test]
    fn hover_jmp() {
        test_hover("jmp 0", 0, "\n## Operation\n\nSet program counter to");
        test_hover("jmp 0", 4, "Is a program label or value");
        test_hover("jmp !x 0", 4, "Is an optional condition listed");
    }

    #[test]
    fn hover_wait() {
        test_hover(
            "wait 0 pin 0",
            0,
            "\n## Operation\n\nStall until some condition",
        );
        test_hover("wait 0 pin 0", 5, "Is a value specifying the polarity");
        test_hover("wait 0 pin 0", 7, "gpio | pin");
        test_hover(
            "wait 0 pin 0",
            11,
            "Is a value specifying the input pin number",
        );
    }

    #[test]
    fn hover_push() {
        test_hover("push", 0, "\n## Operation\n\nPush the contents of");
        test_hover("push block", 5, "Is equivalent to Block == 1");
        test_hover("push noblock", 5, "Is equivalent to Block == 0");
        test_hover("push iffull", 5, "Is equivalent to IfFull == 1");
    }

    #[test]
    fn hover_pull() {
        test_hover("pull", 0, "\n## Operation\n\nLoad a 32-bit word");
        test_hover("pull block", 5, "Is equivalent to Block == 1");
        test_hover("pull noblock", 5, "Is equivalent to Block == 0");
        test_hover("pull ifempty", 5, "Is equivalent to IfEmpty == 1");
    }

    #[test]
    fn hover_in() {
        test_hover("in 1", 0, "\n## Operation\n\nShift Bit count bits from");
        test_hover(
            "in 1",
            3,
            "Is a value specifying the number of bits to shift",
        );
        test_hover("in pins 1", 3, "Is one of the sources");
    }

    #[test]
    fn hover_out() {
        test_hover("out 1", 0, "\n## Operation\n\nShift Bit count bits out");
        test_hover(
            "out 1",
            4,
            "Is a value specifying the number of bits to shift",
        );
        test_hover("out pins 1", 4, "Is one of the destinations");
    }

    #[test]
    fn hover_mov() {
        test_hover(
            "mov x, y",
            0,
            "# MOV\n\n## Operation\n\nCopy data from Source to Destination",
        );
        test_hover("mov x y", 4, "Is one of the destinations specified above.");
        test_hover("mov x :: y", 6, "If present, is:");
        test_hover("mov x y", 6, "Is one of the sources specified above.");
        test_hover(
            "mov rxfifo[y], y",
            4,
            "Is one of the destinations specified above.",
        );
    }

    #[test]
    fn hover_irq() {
        test_hover("irq 1", 0, "\n## Operation\n\nSet or clear the IRQ flag");
        test_hover("irq 1", 4, "Is a value specifying The irq");
        test_hover(
            "irq prev 1",
            4,
            "(version 1 and above) To target the IRQ on the next lower",
        );
        test_hover(
            "irq next 1",
            4,
            "(version 1 and above) To target the IRQ on the next higher",
        );
        test_hover(
            "irq next set 1",
            9,
            "Also means set the IRQ without waiting",
        );
        test_hover(
            "irq nowait 1",
            4,
            "Again, means set the IRQ without waiting",
        );
        test_hover(
            "irq wait 1",
            4,
            "Means set the IRQ and wait for it to be cleared before proceeding",
        );
        test_hover("irq clear 1", 4, "Means clear the IRQ");
    }

    #[test]
    fn hover_set() {
        test_hover(
            "set x 1",
            0,
            "\n## Operation\n\nWrite immediate value Data to Destination.",
        );
        test_hover("set x 1", 4, "Is one of the destinations specified above.");
        test_hover("set x 1", 6, "The value to set");
    }

    #[test]
    fn test_partial_update() {
        let mut doc = r#".program p
nop
set x 23
"#
        .to_owned();

        let changes = [
            (0, 10, 0, 10, "\njmp 3"),
            (2, 0, 3, 0, ""),
            (2, 8, 2, 8, "\nnop"),
            (2, 3, 2, 4, "1"),
            (2, 3, 2, 4, " "),
            (2, 4, 2, 5, "y"),
        ]
        .into_iter()
        .map(|(start_l, start_c, end_l, end_c, text)| {
            TextDocumentContentChangeEvent::TextDocumentContentChangePartial(
                TextDocumentContentChangePartial::new(
                    Range::new(Position::new(start_l, start_c), Position::new(end_l, end_c)),
                    None,
                    text.to_string(),
                ),
            )
        })
        .collect::<Vec<_>>();
        apply_doc_changes(&mut doc, changes);

        let doc_new = r#".program p
jmp 3
set y 23
nop
"#;

        assert_eq!(doc, doc_new);
    }

    #[test]
    fn wrap_get_refs() {
        let doc = doc(r#"
.program p
.wrap_target
    nop
.wrap
"#);

        let pos = Position::new(2, 2);
        let node = doc.node_at(pos).unwrap();

        let (d, refs) = doc.get_references(node).unwrap();
        assert_eq!(ts_range_to_lsp(node.parent().unwrap().range()), d);
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn wrap_get_definition() {
        let doc = doc(r#"
.program p
.wrap_target
    nop
.wrap
"#);

        let pos = Position::new(4, 2);
        let node = doc.node_at(pos).unwrap();

        let sd = doc.get_definition(node).unwrap();

        assert_eq!(sd.typ, SymbolType::Wrap);
        assert_eq!(
            sd.statement_range,
            ts_range_to_lsp(
                doc.node_at(Position::new(2, 2))
                    .unwrap()
                    .parent()
                    .unwrap()
                    .range()
            )
        );
    }
}
