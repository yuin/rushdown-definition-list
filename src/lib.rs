#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use core::any::TypeId;
use core::fmt;
use core::fmt::Write;
use rushdown::as_extension_data;
use rushdown::as_extension_data_mut;
use rushdown::as_type_data;
use rushdown::as_type_data_mut;
use rushdown::ast;
use rushdown::ast::pp_indent;
use rushdown::ast::Arena;
use rushdown::ast::KindData;
use rushdown::ast::NodeKind;
use rushdown::ast::NodeRef;
use rushdown::ast::NodeType;
use rushdown::ast::PrettyPrint;
use rushdown::ast::WalkStatus;
use rushdown::matches_extension_kind;
use rushdown::matches_kind;
use rushdown::parser;
use rushdown::parser::AnyBlockParser;
use rushdown::parser::BlockParser;
use rushdown::parser::NoParserOptions;
use rushdown::parser::Parser;
use rushdown::parser::ParserExtension;
use rushdown::parser::ParserExtensionFn;
use rushdown::renderer;
use rushdown::renderer::html;
use rushdown::renderer::html::ParagraphRendererOptions;
use rushdown::renderer::html::Renderer;
use rushdown::renderer::html::RendererExtension;
use rushdown::renderer::html::RendererExtensionFn;
use rushdown::renderer::BoxRenderNode;
use rushdown::renderer::NoRendererOptions;
use rushdown::renderer::NodeRenderer;
use rushdown::renderer::NodeRendererRegistry;
use rushdown::renderer::RenderNode;
use rushdown::renderer::TextWrite;
use rushdown::text;
use rushdown::text::Reader;
use rushdown::util::indent_position;
use rushdown::util::indent_width;
use rushdown::util::is_blank;
use rushdown::Result;

// AST {{{

/// Represents a definition list in the AST.
#[derive(Debug)]
pub struct DefinitionList {
    offset: u8,
    temp_paragraph: Option<NodeRef>,
    is_tight: bool,
}

impl Default for DefinitionList {
    fn default() -> Self {
        Self::new()
    }
}

impl DefinitionList {
    /// Creates a new `DefinitionList`.
    pub fn new() -> Self {
        Self {
            offset: 0,
            temp_paragraph: None,
            is_tight: true,
        }
    }

    fn with_offset_and_paragraph(offset: u8, temp_paragraph: NodeRef) -> Self {
        Self {
            offset,
            temp_paragraph: Some(temp_paragraph),
            is_tight: true,
        }
    }

    /// Sets whether the definition list is tight or loose.
    #[inline(always)]
    pub fn set_tight(&mut self, tight: bool) {
        self.is_tight = tight;
    }

    /// Returns whether the definition list is tight or loose.
    #[inline(always)]
    pub fn is_tight(&self) -> bool {
        self.is_tight
    }
}

impl NodeKind for DefinitionList {
    fn typ(&self) -> NodeType {
        NodeType::ContainerBlock
    }

    fn kind_name(&self) -> &'static str {
        "DefinitionList"
    }
}

impl PrettyPrint for DefinitionList {
    fn pretty_print(&self, w: &mut dyn Write, _source: &str, level: usize) -> fmt::Result {
        writeln!(w, "{}IsTight: {}", pp_indent(level), self.is_tight)?;
        Ok(())
    }
}

impl From<DefinitionList> for KindData {
    fn from(e: DefinitionList) -> Self {
        KindData::Extension(Box::new(e))
    }
}

/// Represents a term in the AST.
#[derive(Debug)]
pub struct Term {}

impl Default for Term {
    fn default() -> Self {
        Self::new()
    }
}

impl Term {
    /// Creates a new `Term`.
    pub fn new() -> Self {
        Self {}
    }
}

impl NodeKind for Term {
    fn typ(&self) -> NodeType {
        NodeType::LeafBlock
    }

    fn kind_name(&self) -> &'static str {
        "Term"
    }
}

impl PrettyPrint for Term {
    fn pretty_print(&self, _w: &mut dyn Write, _source: &str, _level: usize) -> fmt::Result {
        Ok(())
    }
}

impl From<Term> for KindData {
    fn from(e: Term) -> Self {
        KindData::Extension(Box::new(e))
    }
}

/// Represents a term definition in the AST.
#[derive(Debug)]
pub struct TermDefinition {}

impl Default for TermDefinition {
    fn default() -> Self {
        Self::new()
    }
}

impl TermDefinition {
    /// Creates a new `TermDefinition`.
    pub fn new() -> Self {
        Self {}
    }
}

impl NodeKind for TermDefinition {
    fn typ(&self) -> NodeType {
        NodeType::ContainerBlock
    }

    fn kind_name(&self) -> &'static str {
        "TermDefinition"
    }
}

impl PrettyPrint for TermDefinition {
    fn pretty_print(&self, _w: &mut dyn Write, _source: &str, _level: usize) -> fmt::Result {
        Ok(())
    }
}

impl From<TermDefinition> for KindData {
    fn from(e: TermDefinition) -> Self {
        KindData::Extension(Box::new(e))
    }
}

// }}} AST

// Parser {{{

#[derive(Debug, Default)]
struct DefinitionListParser {}

impl DefinitionListParser {
    fn new() -> Self {
        Self {}
    }
}

impl BlockParser for DefinitionListParser {
    fn trigger(&self) -> &[u8] {
        b":"
    }

    fn open(
        &self,
        arena: &mut Arena,
        parent_ref: NodeRef,
        reader: &mut text::BasicReader,
        ctx: &mut parser::Context,
    ) -> Option<(NodeRef, parser::State)> {
        if matches_extension_kind!(arena, parent_ref, DefinitionList) {
            return None;
        }
        let (line, _) = reader.peek_line_bytes()?;
        let pos = ctx.block_offset()?;
        let indent = ctx.block_indent().unwrap_or(1);
        if line[pos] != b':' || indent != 0 {
            return None;
        }
        let last_ref = arena[parent_ref].last_child()?;
        let (mut w, _) = indent_width(&line[pos + 1..], pos + 1);
        // need 1 or more spaces after ':'
        if w < 1 {
            return None;
        }
        if w > 8 {
            // starts with indented code
            w = 5;
        }
        w += pos + 1; // 1 = ':'

        if matches_kind!(arena, last_ref, Paragraph) {
            match arena[last_ref].previous_sibling() {
                Some(prev_ref) if matches_extension_kind!(arena, prev_ref, DefinitionList) => {
                    // not first item
                    let kd = as_extension_data_mut!(arena, prev_ref, DefinitionList);
                    kd.offset = w as u8;
                    kd.temp_paragraph = Some(last_ref);
                    prev_ref.remove(arena);
                    Some((prev_ref, parser::State::HAS_CHILDREN))
                }
                _ => {
                    // first item
                    let list = DefinitionList::with_offset_and_paragraph(w as u8, last_ref);
                    Some((
                        arena.new_node(list),
                        parser::State::HAS_CHILDREN | parser::State::REQUIRE_PARAGRAPH,
                    ))
                }
            }
        } else if matches_extension_kind!(arena, last_ref, DefinitionList) {
            // multiple definition
            let kd = as_extension_data_mut!(arena, last_ref, DefinitionList);
            kd.offset = w as u8;
            kd.temp_paragraph = None;
            last_ref.remove(arena);
            Some((last_ref, parser::State::HAS_CHILDREN))
        } else {
            None
        }
    }

    fn cont(
        &self,
        arena: &mut Arena,
        node_ref: NodeRef,
        reader: &mut text::BasicReader,
        _ctx: &mut parser::Context,
    ) -> Option<parser::State> {
        let (line, _) = reader.peek_line_bytes()?;
        if is_blank(&line) {
            return Some(parser::State::HAS_CHILDREN);
        }
        let kd = as_extension_data!(arena, node_ref, DefinitionList);
        let w = indent_width(&line, reader.line_offset()).0;
        if w < kd.offset as usize {
            None
        } else {
            let (pos, padding) = indent_position(&line, reader.line_offset(), kd.offset as usize)?;
            reader.advance_and_set_padding(pos, padding);
            Some(parser::State::HAS_CHILDREN)
        }
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }
}

impl From<DefinitionListParser> for AnyBlockParser {
    fn from(p: DefinitionListParser) -> Self {
        AnyBlockParser::Extension(Box::new(p))
    }
}

#[derive(Debug, Default)]
struct TermDefinitionParser {}

impl TermDefinitionParser {
    fn new() -> Self {
        Self {}
    }
}

impl BlockParser for TermDefinitionParser {
    fn trigger(&self) -> &[u8] {
        b":"
    }

    fn open(
        &self,
        arena: &mut Arena,
        parent_ref: NodeRef,
        reader: &mut text::BasicReader,
        ctx: &mut parser::Context,
    ) -> Option<(NodeRef, parser::State)> {
        let (line, _) = reader.peek_line_bytes()?;
        let pos = ctx.block_offset()?;
        let indent = ctx.block_indent().unwrap_or(1);
        if line[pos] != b':' || indent != 0 {
            return None;
        }
        if !matches_extension_kind!(arena, parent_ref, DefinitionList) {
            return None;
        }
        let para_opt = {
            let list_kd = as_extension_data_mut!(arena, parent_ref, DefinitionList);
            let para_opt = list_kd.temp_paragraph;
            list_kd.temp_paragraph = None;
            para_opt
        };
        if let Some(para_ref) = para_opt {
            let para_td = as_type_data_mut!(arena, para_ref, Block);
            let lines = para_td.take_source();
            for line in &lines {
                let term_ref = arena.new_node(Term::new());
                as_type_data_mut!(arena, term_ref, Block)
                    .append_source_line(line.trim_right_space(reader.source()));
                parent_ref.append_child(arena, term_ref);
            }
            para_ref.remove(arena);
        }
        let list_kd = as_extension_data_mut!(arena, parent_ref, DefinitionList);
        let (pos, padding) = indent_position(
            &line[pos + 1..],
            pos + 1,
            (list_kd.offset as usize) - pos - 1,
        )?;
        reader.advance_and_set_padding(pos + 1, padding);
        Some((
            arena.new_node(TermDefinition::new()),
            parser::State::HAS_CHILDREN,
        ))
    }

    fn cont(
        &self,
        _arena: &mut Arena,
        _node_ref: NodeRef,
        _reader: &mut text::BasicReader,
        _ctx: &mut parser::Context,
    ) -> Option<parser::State> {
        // definitionListParser detects end of the description.
        // so this method will never be called.
        Some(parser::State::HAS_CHILDREN)
    }

    fn close(
        &self,
        arena: &mut Arena,
        node_ref: NodeRef,
        _reader: &mut text::BasicReader,
        _ctx: &mut parser::Context,
    ) {
        if as_type_data!(arena, node_ref, Block).has_blank_previous_line() {
            let mut cur = node_ref;
            while let Some(parent_ref) = arena[cur].parent() {
                if matches_extension_kind!(arena, parent_ref, DefinitionList) {
                    let kd = as_extension_data_mut!(arena, parent_ref, DefinitionList);
                    kd.set_tight(false);
                    break;
                }
                cur = parent_ref;
            }
        }
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }
}

impl From<TermDefinitionParser> for AnyBlockParser {
    fn from(p: TermDefinitionParser) -> Self {
        AnyBlockParser::Extension(Box::new(p))
    }
}

// }}}

// Renderer {{{

struct DefinitionListHtmlRenderer<W: TextWrite> {
    _phantom: core::marker::PhantomData<W>,
    writer: html::Writer,
}

impl<W: TextWrite> DefinitionListHtmlRenderer<W> {
    fn new(html_opts: html::Options) -> Self {
        Self {
            _phantom: core::marker::PhantomData,
            writer: html::Writer::with_options(html_opts),
        }
    }
}

impl<W: TextWrite> RenderNode<W> for DefinitionListHtmlRenderer<W> {
    fn render_node<'a>(
        &self,
        w: &mut W,
        _source: &'a str,
        _arena: &'a Arena,
        _node_ref: NodeRef,
        entering: bool,
        _context: &mut renderer::Context,
    ) -> Result<WalkStatus> {
        if entering {
            self.writer.write_safe_str(w, "<dl>\n")?
        } else {
            self.writer.write_safe_str(w, "</dl>\n")?
        }
        Ok(WalkStatus::Continue)
    }
}

impl<'cb, W> NodeRenderer<'cb, W> for DefinitionListHtmlRenderer<W>
where
    W: TextWrite + 'cb,
{
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'cb, W>) {
        nrr.register_node_renderer_fn(TypeId::of::<DefinitionList>(), BoxRenderNode::new(self));
    }
}

struct TermHtmlRenderer<W: TextWrite> {
    _phantom: core::marker::PhantomData<W>,
    writer: html::Writer,
}

impl<W: TextWrite> TermHtmlRenderer<W> {
    fn new(html_opts: html::Options) -> Self {
        Self {
            _phantom: core::marker::PhantomData,
            writer: html::Writer::with_options(html_opts),
        }
    }
}

impl<W: TextWrite> RenderNode<W> for TermHtmlRenderer<W> {
    fn render_node<'a>(
        &self,
        w: &mut W,
        _source: &'a str,
        _arena: &'a Arena,
        _node_ref: NodeRef,
        entering: bool,
        _context: &mut renderer::Context,
    ) -> Result<WalkStatus> {
        if entering {
            self.writer.write_safe_str(w, "<dt>")?
        } else {
            self.writer.write_safe_str(w, "</dt>\n")?
        }
        Ok(WalkStatus::Continue)
    }
}

impl<'cb, W> NodeRenderer<'cb, W> for TermHtmlRenderer<W>
where
    W: TextWrite + 'cb,
{
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'cb, W>) {
        nrr.register_node_renderer_fn(TypeId::of::<Term>(), BoxRenderNode::new(self));
    }
}

struct TermDefinitionHtmlRenderer<W: TextWrite> {
    _phantom: core::marker::PhantomData<W>,
    writer: html::Writer,
}

impl<W: TextWrite> TermDefinitionHtmlRenderer<W> {
    fn new(html_opts: html::Options) -> Self {
        Self {
            _phantom: core::marker::PhantomData,
            writer: html::Writer::with_options(html_opts),
        }
    }
}

impl<W: TextWrite> RenderNode<W> for TermDefinitionHtmlRenderer<W> {
    fn render_node<'a>(
        &self,
        w: &mut W,
        _source: &'a str,
        arena: &'a Arena,
        node_ref: NodeRef,
        entering: bool,
        _context: &mut renderer::Context,
    ) -> Result<WalkStatus> {
        if entering {
            self.writer.write_safe_str(w, "<dd>")?;
            if let Some(p) = arena[node_ref].parent() {
                if matches_extension_kind!(arena, p, DefinitionList) {
                    let kd = as_extension_data!(arena, p, DefinitionList);
                    if !kd.is_tight() {
                        self.writer.write_safe_str(w, "\n")?;
                    }
                }
            }
        } else {
            self.writer.write_safe_str(w, "</dd>\n")?
        }
        Ok(WalkStatus::Continue)
    }
}

impl<'cb, W> NodeRenderer<'cb, W> for TermDefinitionHtmlRenderer<W>
where
    W: TextWrite + 'cb,
{
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'cb, W>) {
        nrr.register_node_renderer_fn(TypeId::of::<TermDefinition>(), BoxRenderNode::new(self));
    }
}

/// Returns true if the given node is a child of a tight list, otherwise false.
#[inline(always)]
pub fn is_in_tight_list(arena: &ast::Arena, node_ref: ast::NodeRef) -> bool {
    if let Some(p) = arena[node_ref].parent() {
        if let Some(gp) = arena[p].parent() {
            if matches_extension_kind!(arena, gp, DefinitionList) {
                let kd = as_extension_data!(arena, gp, DefinitionList);
                return kd.is_tight();
            }
        }
    }
    html::is_in_tight_list(arena, node_ref)
}

// }}} Renderer

// Extension {{{

/// Returns a parser extension that parses definition lists.
pub fn definition_list_parser_extension() -> impl ParserExtension {
    ParserExtensionFn::new(|p: &mut Parser| {
        p.add_block_parser(DefinitionListParser::new, NoParserOptions, 101);
        p.add_block_parser(TermDefinitionParser::new, NoParserOptions, 102);
    })
}

/// Returns a renderer extension that renders definition lists as HTML.
pub fn definition_list_html_renderer_extension<'cb, W>() -> impl RendererExtension<'cb, W>
where
    W: TextWrite + 'cb,
{
    RendererExtensionFn::new(move |r: &mut Renderer<'cb, W>| {
        r.add_node_renderer(DefinitionListHtmlRenderer::new, NoRendererOptions);
        r.add_node_renderer(TermDefinitionHtmlRenderer::new, NoRendererOptions);
        r.add_node_renderer(TermHtmlRenderer::new, NoRendererOptions);
    })
    .and(html::paragraph_renderer(ParagraphRendererOptions::<W> {
        is_in_tight_block: Some(is_in_tight_list),
        ..Default::default()
    }))
}

// }}}
