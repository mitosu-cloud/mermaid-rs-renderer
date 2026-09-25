use anyhow::{Result, bail};

use crate::ir::{DiagramKind, Graph, RailroadExpr, RailroadRule};

use super::{ParseOutput, preprocess_input};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String),
    Quoted(String),
    Symbol(char),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialect {
    Ebnf,
    Abnf,
    Peg,
    Ir,
}

pub(super) fn parse_railroad(input: &str) -> Result<ParseOutput> {
    let (lines, init_config) = preprocess_input(input)?;
    let mut graph = Graph::new();
    graph.kind = DiagramKind::Railroad;
    let mut body = String::new();
    let mut dialect = None;
    for line in lines {
        let line = line.trim();
        if dialect.is_none() {
            dialect = Some(match line {
                "railroad-ebnf-beta" => Dialect::Ebnf,
                "railroad-abnf-beta" => Dialect::Abnf,
                "railroad-peg-beta" => Dialect::Peg,
                "railroad-beta" => Dialect::Ir,
                _ => bail!("expected railroad diagram header"),
            });
        } else if let Some(title) = line.strip_prefix("title ") {
            graph.railroad.title = Some(title.trim().to_string());
        } else if let Some(title) = line.strip_prefix("accTitle:") {
            graph.acc_title = Some(title.trim().to_string());
        } else if let Some(description) = line.strip_prefix("accDescr:") {
            graph.acc_descr = Some(description.trim().to_string());
        } else {
            body.push_str(line);
            body.push(' ');
        }
    }
    let Some(dialect) = dialect else {
        bail!("missing railroad diagram header")
    };
    let tokens = lex(&body)?;
    let mut parser = Parser {
        tokens,
        position: 0,
    };
    while parser.position < parser.tokens.len() {
        let name = match parser.next() {
            Some(Token::Word(name)) => name,
            other => bail!("expected railroad rule name, found {other:?}"),
        };
        if dialect == Dialect::Peg {
            if parser.next() != Some(Token::Word("<-".to_string())) {
                bail!("expected '<-' in PEG railroad rule");
            }
        } else {
            parser.expect('=')?;
        }
        let expression = if dialect == Dialect::Ir {
            parser.ir_expression(0)?
        } else {
            parser.expression(0, dialect)?
        };
        parser.expect(';')?;
        graph.railroad.rules.push(RailroadRule { name, expression });
    }
    if graph.railroad.rules.is_empty() {
        bail!("railroad diagram has no rules");
    }
    Ok(ParseOutput { graph, init_config })
}

fn lex(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&next) = chars.peek() {
        if next.is_whitespace() {
            chars.next();
        } else if next == '\'' || next == '"' {
            let quote = chars.next().unwrap();
            let mut value = String::new();
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == quote {
                    closed = true;
                    break;
                }
                if c == '\\' {
                    let Some(escaped) = chars.next() else {
                        bail!("unfinished escape in railroad terminal");
                    };
                    value.push(escaped);
                } else {
                    value.push(c);
                }
            }
            if !closed {
                bail!("unterminated railroad terminal");
            }
            tokens.push(Token::Quoted(value));
        } else if "=,|/;()[]{}?*+&!".contains(next) {
            tokens.push(Token::Symbol(chars.next().unwrap()));
        } else {
            let mut word = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || "=,|/;()[]{}?*+&!'\"".contains(c) {
                    break;
                }
                word.push(c);
                chars.next();
            }
            if word.is_empty() {
                bail!("unexpected character in railroad rule: {next}");
            }
            tokens.push(Token::Word(word));
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.peek()?.clone();
        self.position += 1;
        Some(token)
    }

    fn expect(&mut self, symbol: char) -> Result<()> {
        if self.next() == Some(Token::Symbol(symbol)) {
            Ok(())
        } else {
            bail!("expected '{symbol}' in railroad rule")
        }
    }

    fn expression(&mut self, depth: usize, dialect: Dialect) -> Result<RailroadExpr> {
        if depth > 64 {
            bail!("railroad expression nesting is too deep");
        }
        let choice_symbol = if dialect == Dialect::Ebnf { '|' } else { '/' };
        let mut options = vec![self.sequence(depth, dialect)?];
        while self.peek() == Some(&Token::Symbol(choice_symbol)) {
            self.next();
            options.push(self.sequence(depth, dialect)?);
        }
        Ok(if options.len() == 1 {
            options.remove(0)
        } else {
            RailroadExpr::Choice(options)
        })
    }

    fn sequence(&mut self, depth: usize, dialect: Dialect) -> Result<RailroadExpr> {
        let mut elements = Vec::new();
        loop {
            if dialect == Dialect::Ebnf && self.peek() == Some(&Token::Symbol(',')) {
                self.next();
                continue;
            }
            let starts_element = match self.peek() {
                Some(Token::Word(_) | Token::Quoted(_)) => true,
                Some(Token::Symbol('(' | '[')) => true,
                Some(Token::Symbol('{')) => dialect == Dialect::Ebnf,
                Some(Token::Symbol('*')) => dialect == Dialect::Abnf,
                _ => false,
            };
            if !starts_element {
                break;
            }
            elements.push(self.element(depth, dialect)?);
        }
        if elements.is_empty() {
            bail!("empty railroad expression");
        }
        Ok(if elements.len() == 1 {
            elements.remove(0)
        } else {
            RailroadExpr::Sequence(elements)
        })
    }

    fn element(&mut self, depth: usize, dialect: Dialect) -> Result<RailroadExpr> {
        if dialect == Dialect::Abnf && self.peek() == Some(&Token::Symbol('*')) {
            self.next();
            return Ok(RailroadExpr::ZeroOrMore(Box::new(
                self.element(depth + 1, dialect)?,
            )));
        }
        if dialect == Dialect::Abnf
            && self.peek() == Some(&Token::Word("1".to_string()))
            && self.tokens.get(self.position + 1) == Some(&Token::Symbol('*'))
        {
            self.next();
            self.next();
            return Ok(RailroadExpr::OneOrMore(Box::new(
                self.element(depth + 1, dialect)?,
            )));
        }
        let mut expression = match self.next() {
            Some(Token::Word(word)) if word.starts_with('%') && dialect == Dialect::Abnf => {
                RailroadExpr::Terminal(word)
            }
            Some(Token::Word(word))
                if dialect == Dialect::Abnf
                    && word.chars().next().is_some_and(|c| c.is_ascii_digit()) =>
            {
                bail!("unsupported ABNF repetition count: {word}")
            }
            Some(Token::Word(word)) if word == "." && dialect == Dialect::Peg => {
                RailroadExpr::Terminal("any character".to_string())
            }
            Some(Token::Word(word)) => RailroadExpr::Nonterminal(word),
            Some(Token::Quoted(text)) => RailroadExpr::Terminal(text),
            Some(Token::Symbol('(')) => {
                let expression = self.expression(depth + 1, dialect)?;
                self.expect(')')?;
                expression
            }
            Some(Token::Symbol('[')) if dialect != Dialect::Peg => {
                let expression = self.expression(depth + 1, dialect)?;
                self.expect(']')?;
                RailroadExpr::Optional(Box::new(expression))
            }
            Some(Token::Symbol('{')) if dialect == Dialect::Ebnf => {
                let expression = self.expression(depth + 1, dialect)?;
                self.expect('}')?;
                RailroadExpr::ZeroOrMore(Box::new(expression))
            }
            other => bail!("expected railroad element, found {other:?}"),
        };
        if dialect != Dialect::Abnf
            && let Some(Token::Symbol(symbol @ ('?' | '*' | '+'))) = self.peek()
        {
            let symbol = *symbol;
            self.next();
            expression = match symbol {
                '?' => RailroadExpr::Optional(Box::new(expression)),
                '*' => RailroadExpr::ZeroOrMore(Box::new(expression)),
                '+' => RailroadExpr::OneOrMore(Box::new(expression)),
                _ => unreachable!(),
            };
        }
        Ok(expression)
    }

    fn ir_expression(&mut self, depth: usize) -> Result<RailroadExpr> {
        if depth > 64 {
            bail!("railroad expression nesting is too deep");
        }
        let name = match self.next() {
            Some(Token::Word(name)) => name,
            other => bail!("expected railroad constructor, found {other:?}"),
        };
        self.expect('(')?;
        let result = match name.as_str() {
            "terminal" | "nonterminal" | "special" => {
                let text = match self.next() {
                    Some(Token::Quoted(text)) => text,
                    other => bail!("expected quoted railroad label, found {other:?}"),
                };
                if name == "nonterminal" {
                    RailroadExpr::Nonterminal(text)
                } else {
                    RailroadExpr::Terminal(text)
                }
            }
            "sequence" | "choice" => {
                let mut items = vec![self.ir_expression(depth + 1)?];
                while self.peek() == Some(&Token::Symbol(',')) {
                    self.next();
                    items.push(self.ir_expression(depth + 1)?);
                }
                if name == "sequence" {
                    RailroadExpr::Sequence(items)
                } else {
                    RailroadExpr::Choice(items)
                }
            }
            "optional" | "zeroOrMore" | "oneOrMore" => {
                let inner = Box::new(self.ir_expression(depth + 1)?);
                match name.as_str() {
                    "optional" => RailroadExpr::Optional(inner),
                    "zeroOrMore" => RailroadExpr::ZeroOrMore(inner),
                    _ => RailroadExpr::OneOrMore(inner),
                }
            }
            _ => bail!("unsupported railroad constructor: {name}"),
        };
        self.expect(')')?;
        Ok(result)
    }
}
