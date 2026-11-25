//! Explicitly typed when AST; entry points establish syntax/node-count/recursion-depth guarantees together.
use super::{lexer::Token, Error, Expr, Value};
struct Parser {
    tokens: std::iter::Peekable<std::vec::IntoIter<Token>>,
    nodes: usize,
    unsupported: bool,
}
pub(super) fn parse(tokens: Vec<Token>) -> Result<(Expr, bool), Error> {
    let mut parser = Parser {
        tokens: tokens.into_iter().peekable(),
        nodes: 0,
        unsupported: false,
    };
    let expression = parser.or(0)?;
    if parser.tokens.next() != Some(Token::End) {
        return Err(Error::Syntax);
    }
    Ok((expression, parser.unsupported))
}
impl Parser {
    fn take(&mut self, operator: &str) -> bool {
        if self.tokens.peek().is_some_and(|t| t.value() == operator) {
            self.tokens.next();
            true
        } else {
            false
        }
    }
    fn expect(&mut self, operator: &str) -> Result<(), Error> {
        if self.take(operator) {
            Ok(())
        } else {
            Err(Error::Syntax)
        }
    }
    fn node(&mut self, node: Expr, depth: usize) -> Result<Expr, Error> {
        self.nodes += 1;
        if depth > 16 || self.nodes > 128 {
            return Err(Error::Budget);
        }
        Ok(node)
    }
    fn or(&mut self, depth: usize) -> Result<Expr, Error> {
        let mut left = self.and(depth + 1)?;
        while self.take("||") {
            let right = self.and(depth + 1)?;
            left = self.node(Expr::Or(Box::new(left), Box::new(right)), depth)?;
        }
        Ok(left)
    }
    fn and(&mut self, depth: usize) -> Result<Expr, Error> {
        let mut left = self.comparison(depth + 1)?;
        while self.take("&&") {
            let right = self.comparison(depth + 1)?;
            left = self.node(Expr::And(Box::new(left), Box::new(right)), depth)?;
        }
        Ok(left)
    }
    fn comparison(&mut self, depth: usize) -> Result<Expr, Error> {
        // Check before descending so malicious nested parentheses/negations cannot exhaust stacks before node checks.
        if depth > 16 {
            return Err(Error::Budget);
        }
        if self.take("!") {
            let operand = self.comparison(depth + 1)?;
            return self.node(Expr::Not(Box::new(operand)), depth);
        }
        if self.take("(") {
            let expression = self.or(depth + 1)?;
            self.expect(")")?;
            return Ok(expression);
        }
        if self
            .tokens
            .peek()
            .is_some_and(|t| t.value() == "attachment.has")
        {
            self.tokens.next();
            self.expect("(")?;
            let Some(Token::Text(id)) = self.tokens.next() else {
                return Err(Error::Syntax);
            };
            if id.is_empty() {
                return Err(Error::Syntax);
            }
            self.expect(")")?;
            return self.node(Expr::Attachment(id), depth);
        }
        let left = self.value()?;
        let equal = if self.take("==") {
            Some(true)
        } else if self.take("!=") {
            Some(false)
        } else {
            None
        };
        let node = match equal {
            Some(equal) => Expr::Equal(left, self.value()?, equal),
            None => Expr::Value(left),
        };
        self.node(node, depth)
    }
    fn value(&mut self) -> Result<Value, Error> {
        Ok(match self.tokens.next() {
            Some(Token::Text(text)) => Value::Text(text),
            Some(Token::Bool(value)) => Value::Bool(value),
            Some(Token::Identifier(id)) => match id.as_str() {
                "unit.kind" => Value::Kind,
                "unit.hasImage" => Value::Image,
                _ => {
                    self.unsupported = true;
                    Value::Bool(false)
                }
            },
            _ => return Err(Error::Syntax),
        })
    }
}
