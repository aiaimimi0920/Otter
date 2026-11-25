//! 主干扩展 when 的有界纯表达式；未知属性隐藏整项，不执行脚本或访问宿主数据。
#[path = "extension_when_lexer.rs"]
pub mod lexer;
#[path = "extension_when_parser.rs"]
pub mod parser;
#[cfg(test)]
#[path = "extension_when_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Budget,
    Token,
    Syntax,
    Type,
}
#[derive(Clone, Copy)]
pub enum UnitKind {
    Sticker,
    Art,
}
pub struct Context<'a> {
    pub kind: Option<UnitKind>,
    pub has_image: bool,
    pub attachment_types: &'a [&'a str],
}
#[derive(Debug)]
enum Value {
    Text(String),
    Bool(bool),
    Kind,
    Image,
}
#[derive(Debug)]
enum Expr {
    Value(Value),
    Attachment(String),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Equal(Value, Value, bool),
}
#[derive(Debug)]
pub struct Predicate(Expr);
#[derive(PartialEq)]
enum Scalar<'a> {
    Text(&'a str),
    Bool(bool),
    Null,
}
impl Value {
    fn boolean(&self) -> bool {
        matches!(self, Self::Bool(_) | Self::Image)
    }
    fn get<'a>(&'a self, context: &Context<'_>) -> Scalar<'a> {
        match self {
            Self::Text(text) => Scalar::Text(text),
            Self::Bool(value) => Scalar::Bool(*value),
            Self::Image => Scalar::Bool(context.has_image),
            Self::Kind => match context.kind {
                Some(UnitKind::Sticker) => Scalar::Text("sticker"),
                Some(UnitKind::Art) => Scalar::Text("art"),
                None => Scalar::Null,
            },
        }
    }
}
impl Expr {
    fn validate(&self) -> Result<(), Error> {
        match self {
            Self::Value(value) if !value.boolean() => Err(Error::Type),
            Self::Equal(a, b, _) if a.boolean() != b.boolean() => Err(Error::Type),
            Self::Not(value) => value.validate(),
            Self::And(a, b) | Self::Or(a, b) => {
                a.validate()?;
                b.validate()
            }
            _ => Ok(()),
        }
    }
    fn evaluate(&self, context: &Context<'_>) -> bool {
        match self {
            Self::Value(value) => value.get(context) == Scalar::Bool(true),
            Self::Attachment(id) => context.attachment_types.contains(&id.as_str()),
            Self::Not(value) => !value.evaluate(context),
            Self::And(a, b) => a.evaluate(context) && b.evaluate(context),
            Self::Or(a, b) => a.evaluate(context) || b.evaluate(context),
            Self::Equal(a, b, equal) => (a.get(context) == b.get(context)) == *equal,
        }
    }
}
impl Predicate {
    pub fn compile(source: Option<&str>) -> Result<Self, Error> {
        let source = source.unwrap_or("");
        if source.trim_matches(lexer::space).is_empty() {
            return Ok(Self(Expr::Value(Value::Bool(true))));
        }
        let (expression, unsupported) = parser::parse(lexer::tokenize(source)?)?;
        if unsupported {
            return Ok(Self(Expr::Value(Value::Bool(false))));
        }
        expression.validate()?;
        Ok(Self(expression))
    }
    pub fn evaluate(&self, context: &Context<'_>) -> bool {
        self.0.evaluate(context)
    }
}
