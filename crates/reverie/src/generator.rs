use crate::{
    error::{Error, Span},
    parser::{Argument, Ast, Node, NodeId},
};

pub struct Generator<'b> {
    bytes: &'b [u8],
    ast: Ast,
}

impl<'b> Generator<'b> {
    pub fn new(bytes: &'b [u8], ast: Ast) -> Self {
        Self { bytes, ast }
    }

    fn group(&self, output: &mut Vec<u8>, nodes: &[NodeId]) -> Result<(), Error> {
        for &node in nodes {
            self.node(output, node)?;
        }
        Ok(())
    }

    fn one_wrapped_in(&self, output: &mut Vec<u8>, node: NodeId, tag: &[u8]) -> Result<(), Error> {
        self.many_wrapped_in(output, &[node], tag)
    }

    fn many_wrapped_in(
        &self,
        output: &mut Vec<u8>,
        nodes: &[NodeId],
        tag: &[u8],
    ) -> Result<(), Error> {
        output.push(b'<');
        output.extend_from_slice(tag);
        output.push(b'>');
        self.group(output, nodes)?;
        output.extend_from_slice(b"</");
        output.extend_from_slice(tag);
        output.push(b'>');
        Ok(())
    }

    fn command(&self, output: &mut Vec<u8>, name: Span, args: &[Argument]) -> Result<(), Error> {
        match &self.bytes[name] {
            b"i" => self.italic(output, name, args)?,
            b"b" => self.bold(output, name, args)?,
            b"section" => self.section(output, name, args)?,
            b"subsection" => self.subsection(output, name, args)?,
            b"super" => self.super_(output, name, args)?,
            b"code" => self.code(output, name, args)?,
            b"link" => self.link(output, name, args)?,
            b"aside" => self.aside(output, name, args)?,
            b"blockquote" => self.blockquote(output, name, args)?,
            b"codeblock" => self.codeblock(output, name, args)?,
            _ => todo!(),
        }
        Ok(())
    }

    fn italic(&self, output: &mut Vec<u8>, name: Span, args: &[Argument]) -> Result<(), Error> {
        expect_args(name, args, 1)?;
        let node = inline_arg(args, 0)?;
        self.one_wrapped_in(output, node, b"em")?;
        Ok(())
    }

    fn bold(&self, output: &mut Vec<u8>, name: Span, args: &[Argument]) -> Result<(), Error> {
        expect_args(name, args, 1)?;
        let node = inline_arg(args, 0)?;
        self.one_wrapped_in(output, node, b"strong  ")?;
        Ok(())
    }

    fn section(&self, output: &mut Vec<u8>, name: Span, args: &[Argument]) -> Result<(), Error> {
        expect_args(name, args, 1)?;
        let node = inline_arg(args, 0)?;
        self.one_wrapped_in(output, node, b"h2")?;
        Ok(())
    }

    fn subsection(&self, output: &mut Vec<u8>, name: Span, args: &[Argument]) -> Result<(), Error> {
        expect_args(name, args, 1)?;
        let node = inline_arg(args, 0)?;
        self.one_wrapped_in(output, node, b"h3")?;
        Ok(())
    }

    fn super_(&self, output: &mut Vec<u8>, name: Span, args: &[Argument]) -> Result<(), Error> {
        expect_args(name, args, 1)?;
        let node = inline_arg(args, 0)?;
        self.one_wrapped_in(output, node, b"sup")?;
        Ok(())
    }

    fn code(&self, output: &mut Vec<u8>, name: Span, args: &[Argument]) -> Result<(), Error> {
        expect_args(name, args, 1)?;
        let code =
            std::str::from_utf8(&self.bytes[string_arg(args, 0)?]).unwrap().replace("\\\"", "\"");
        let escaped = html_escape::encode_text(&code);
        output.extend_from_slice(b"<code>");
        output.extend_from_slice(escaped.as_bytes());
        output.extend_from_slice(b"</code>");
        Ok(())
    }

    fn link(&self, output: &mut Vec<u8>, name: Span, args: &[Argument]) -> Result<(), Error> {
        expect_args(name, args, 2)?;
        let text = inline_arg(args, 0)?;
        let href = string_arg(args, 1)?;
        output.extend_from_slice(b"<a href=\"");
        output.extend_from_slice(&self.bytes[href]);
        output.extend_from_slice(b"\">");
        self.node(output, text)?;
        output.extend_from_slice(b"</a>");
        Ok(())
    }

    fn aside(&self, output: &mut Vec<u8>, name: Span, args: &[Argument]) -> Result<(), Error> {
        expect_args(name, args, 1)?;
        let nodes = block_arg(args, 0)?;
        self.many_wrapped_in(output, nodes, b"aside")?;
        Ok(())
    }

    fn blockquote(&self, output: &mut Vec<u8>, name: Span, args: &[Argument]) -> Result<(), Error> {
        expect_args(name, args, 1)?;
        let nodes = block_arg(args, 0)?;
        self.many_wrapped_in(output, nodes, b"blockquote")?;
        Ok(())
    }

    fn codeblock(&self, output: &mut Vec<u8>, name: Span, args: &[Argument]) -> Result<(), Error> {
        expect_args(name, args, 2)?;
        let file =
            std::str::from_utf8(&self.bytes[string_arg(args, 0)?]).unwrap().replace("\\\"", "\"");
        let Some((_, ext)) = file.rsplit_once('.') else { panic!("file name {file} has no extension") };
        let name = match ext {
            "rs" => b"Rust",
            "txt" => b"Text",
            _ => panic!("unknown file ext {ext} in {file}"),
        };
        let code =
            std::str::from_utf8(&self.bytes[string_arg(args, 1)?]).unwrap().replace("\\\"", "\"");
        let escaped = html_escape::encode_text(&code);
        output.extend_from_slice(b"<pre><div class=\"language-tag\">");
        output.extend_from_slice(name);
        output.extend_from_slice(b" &bull; ");
        output.extend_from_slice(file.as_bytes());
        output.extend_from_slice(b"</div>");
        for line in escaped.lines() {
            output.extend_from_slice(b"<code>");
            output.extend_from_slice(line.as_bytes());
            output.extend_from_slice(b"</code>");
        }
        output.extend_from_slice(b"</pre>");
        Ok(())
    }

    fn node(&self, output: &mut Vec<u8>, node: NodeId) -> Result<(), Error> {
        match &self.ast[node] {
            Node::Group(nodes) => self.group(output, nodes)?,
            Node::Paragraph(nodes) => self.many_wrapped_in(output, nodes, b"p")?,
            Node::Text(span) => output.extend_from_slice(&self.bytes[*span]),
            Node::Command(name, args) => self.command(output, *name, args)?,
        }
        Ok(())
    }

    pub fn generate(&self) -> Result<Vec<u8>, Error> {
        let mut output = Vec::new();
        self.node(&mut output, self.ast.root)?;
        Ok(output)
    }
}

fn expect_args(name: Span, args: &[Argument], expected: usize) -> Result<(), Error> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(Error::IncorrectArgCount { expected, found: args.len(), span: name })
    }
}

fn inline_arg(args: &[Argument], idx: usize) -> Result<NodeId, Error> {
    match &args[idx] {
        Argument::Inline(node, _) => Ok(*node),
        Argument::Block(_, span) | Argument::Ident(span) | Argument::String(span) => {
            Err(Error::ExpectedInlineArg { span: *span })
        }
    }
}

fn block_arg(args: &[Argument], idx: usize) -> Result<&[NodeId], Error> {
    match &args[idx] {
        Argument::Block(node, _) => Ok(node),
        Argument::Inline(_, span) | Argument::Ident(span) | Argument::String(span) => {
            Err(Error::ExpectedBlockArg { span: *span })
        }
    }
}

// fn ident_arg(args: &[Argument], idx: usize) -> Result<Span, Error> {
//     match &args[idx] {
//         Argument::Ident(span) => Ok(*span),
//         Argument::Inline(_, span) | Argument::Block(_, span) | Argument::String(span) => {
//             Err(Error::ExpectedIdentArg { span: *span })
//         }
//     }
// }

fn string_arg(args: &[Argument], idx: usize) -> Result<Span, Error> {
    match &args[idx] {
        Argument::String(span) => Ok(Span::new(span.start + 1, span.end - 1)),
        Argument::Inline(_, span) | Argument::Block(_, span) | Argument::Ident(span) => {
            Err(Error::ExpectedStringArg { span: *span })
        }
    }
}
