use std::ops::Index;

use crate::{error::Span, Error};

pub struct Ast {
    pub root: NodeId,
    nodes: Vec<Node>,
}

impl Ast {
    fn new() -> Self {
        Self { root: NodeId { idx: 0 }, nodes: Vec::new() }
    }

    fn push(&mut self, node: Node) -> NodeId {
        let idx = self.nodes.len() as u32;
        self.nodes.push(node);
        NodeId { idx }
    }
}

impl Index<NodeId> for Ast {
    type Output = Node;

    fn index(&self, index: NodeId) -> &Node {
        &self.nodes[index.idx as usize]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NodeId {
    idx: u32,
}

pub enum Node {
    Group(Vec<NodeId>),
    Paragraph(Vec<NodeId>),
    Text(Span),
    Command(Span, Vec<Argument>),
}

pub enum Argument {
    Block(Vec<NodeId>, Span),
    Inline(NodeId, Span),
    Ident(Span),
    String(Span),
}

#[derive(Debug, PartialEq, Eq)]
enum Delim {
    Paren,
    Bracket,
    Brace,
}

pub struct Parser<'b> {
    ast: Ast,
    bytes: &'b [u8],
    position: u32,
    delim_stack: Vec<Delim>,
}

impl<'b> Parser<'b> {
    pub fn new(bytes: &'b [u8]) -> Self {
        Self { ast: Ast::new(), bytes, position: 0, delim_stack: Vec::new() }
    }

    fn at_end(&self) -> bool {
        self.position as usize == self.bytes.len()
    }

    fn previous(&mut self) -> u8 {
        self.bytes.get(self.position as usize - 1).copied().unwrap_or(0)
    }

    fn peek(&mut self) -> u8 {
        self.bytes.get(self.position as usize).copied().unwrap_or(0)
    }

    fn bump(&mut self) {
        match self.peek() {
            b'(' => self.delim_stack.push(Delim::Paren),
            b'[' => self.delim_stack.push(Delim::Bracket),
            b'{' => self.delim_stack.push(Delim::Brace),
            b')' => assert_eq!(self.delim_stack.pop(), Some(Delim::Paren)),
            b']' => assert_eq!(self.delim_stack.pop(), Some(Delim::Bracket)),
            b'}' => assert_eq!(self.delim_stack.pop(), Some(Delim::Brace)),
            _ => {}
        }
        self.position += 1;
    }

    fn eat_whitespace(&mut self) {
        while !self.at_end() && self.peek().is_ascii_whitespace() {
            self.bump();
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), Error> {
        let found = self.peek();
        if expected == found {
            self.bump();
            Ok(())
        } else {
            let start = self.position;
            self.bump();
            let end = self.position;
            Err(Error::ExpectedFound { expected, found, span: Span::new(start, end) })
        }
    }

    fn parse_parenthesized_args(&mut self, blocks: bool) -> Result<Vec<Argument>, Error> {
        let mut args = Vec::new();
        self.bump();
        self.eat_whitespace();
        while !self.at_end() {
            self.eat_whitespace();
            match self.peek() {
                b')' => break,
                b'[' => args.push(self.parse_bracketed_arg()?),
                b'{' if blocks => args.push(self.parse_braced_arg()?),
                b'"' => args.push(self.parse_string_arg()),
                b if b.is_ascii_alphabetic() => args.push(self.parse_ident_arg()),
                found => {
                    return Err(Error::ExpectedArgs {
                        found,
                        span: Span::new(self.position, self.position + 1),
                    })
                }
            }
            self.eat_whitespace();
            if self.peek() != b')' {
                self.expect(b',')?;
            }
        }
        self.expect(b')')?;
        Ok(args)
    }

    fn parse_bracketed_arg(&mut self) -> Result<Argument, Error> {
        let start = self.position;
        self.bump();
        let node = self.parse_inline()?;
        self.expect(b']')?;
        Ok(Argument::Inline(node, Span::new(start, self.position)))
    }

    fn parse_braced_arg(&mut self) -> Result<Argument, Error> {
        let start = self.position;
        self.bump();
        let mut nodes = Vec::new();
        loop {
            self.eat_whitespace();
            if self.at_end() || self.peek() == b'}' {
                break;
            }
            nodes.push(self.parse_block()?);
        }
        self.expect(b'}')?;
        Ok(Argument::Block(nodes, Span::new(start, self.position)))
    }

    fn parse_ident_arg(&mut self) -> Argument {
        let start = self.position;
        while self.peek().is_ascii_alphabetic() {
            self.bump();
        }
        Argument::Ident(Span::new(start, self.position))
    }

    fn parse_string_arg(&mut self) -> Argument {
        let start = self.position;
        self.bump();
        while (self.peek() != b'"' || self.previous() == b'\\') && !self.at_end() {
            self.bump();
        }
        self.bump();
        Argument::String(Span::new(start, self.position))
    }

    fn parse_command(&mut self, blocks: bool) -> Result<NodeId, Error> {
        self.bump();
        let name_start = self.position;
        while self.peek().is_ascii_alphabetic() {
            self.bump();
        }
        let name_span = Span::new(name_start, self.position);
        self.eat_whitespace();
        let args = match self.peek() {
            b'(' => self.parse_parenthesized_args(blocks)?,
            b'[' => vec![self.parse_bracketed_arg()?],
            b'{' if blocks => vec![self.parse_braced_arg()?],
            found => {
                return Err(Error::ExpectedArgs {
                    found,
                    span: Span::new(self.position, self.position + 1),
                })
            }
        };
        Ok(self.ast.push(Node::Command(name_span, args)))
    }

    fn parse_text_section(&mut self) -> Result<Vec<NodeId>, Error> {
        let mut current_text_start = Some(self.position);
        let mut nodes = Vec::new();
        let stack_len = self.delim_stack.len();
        while !self.at_end() {
            match self.peek() {
                b'~' => {
                    if let Some(start) = current_text_start {
                        nodes.push(self.ast.push(Node::Text(Span::new(start, self.position))));
                    }
                    current_text_start = None;
                    nodes.push(self.parse_command(false)?);
                }
                b'\n' => {
                    self.bump();
                    if let Some(start) = current_text_start {
                        nodes.push(self.ast.push(Node::Text(Span::new(start, self.position))));
                    }
                    current_text_start = None;
                    break;
                }
                b')' | b']' | b'}' if self.delim_stack.len() == stack_len => {
                    if let Some(start) = current_text_start {
                        nodes.push(self.ast.push(Node::Text(Span::new(start, self.position))));
                    }
                    current_text_start = None;
                    break;
                }
                _ => {
                    if current_text_start.is_none() {
                        current_text_start = Some(self.position);
                    }
                    self.bump();
                }
            }
        }
        if let Some(start) = current_text_start {
            nodes.push(self.ast.push(Node::Text(Span::new(start, self.position))));
        }
        Ok(nodes)
    }

    fn parse_inline(&mut self) -> Result<NodeId, Error> {
        if self.peek() == b'~' {
            self.parse_command(false)
        } else {
            let nodes = self.parse_text_section()?;
            Ok(self.ast.push(Node::Group(nodes)))
        }
    }

    fn parse_block(&mut self) -> Result<NodeId, Error> {
        if self.peek() == b'~' {
            self.parse_command(true)
        } else {
            let nodes = self.parse_text_section()?;
            Ok(self.ast.push(Node::Paragraph(nodes)))
        }
    }

    pub fn parse(mut self) -> Result<Ast, Error> {
        let mut nodes = Vec::new();
        loop {
            self.eat_whitespace();
            if self.at_end() {
                break;
            }
            nodes.push(self.parse_block()?);
        }
        let root = self.ast.push(Node::Group(nodes));
        self.ast.root = root;
        Ok(self.ast)
    }
}
