#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Let {
        name: String,
        name_span: Span,
        value: Expr,
        span: Span,
    },
    Assign {
        name: String,
        name_span: Span,
        value: Expr,
        span: Span,
    },
    Print {
        value: Expr,
        span: Span,
    },
    Expr {
        value: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    BuiltinCall(BuiltinCallExpr),
    DestructiveAssign(DestructiveAssignExpr),
    LetIn(LetInExpr),
    Block(BlockExpr),
    Literal { value: Literal, span: Span },
    Variable { name: String, span: Span },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Binary(binary) => binary.span,
            Expr::Unary(unary) => unary.span,
            Expr::BuiltinCall(call) => call.span,
            Expr::DestructiveAssign(assign) => assign.span,
            Expr::LetIn(let_in) => let_in.span,
            Expr::Block(block) => block.span,
            Expr::Literal { span, .. } => *span,
            Expr::Variable { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub op: BinaryOp,
    pub right: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub expr: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct DestructiveAssignExpr {
    pub name: String,
    pub name_span: Span,
    pub value: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BuiltinCallExpr {
    pub function: BuiltinFunction,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BlockExpr {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LetInExpr {
    pub bindings: Vec<LetBinding>,
    pub body: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Pow,
    Concat,
    Sub,
    Mul,
    Div,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinFunction {
    Print,
    Sin,
    Cos,
    Sqrt,
    Exp,
    Log,
    Rand,
}

impl BuiltinFunction {
    pub const fn name(self) -> &'static str {
        match self {
            BuiltinFunction::Print => "print",
            BuiltinFunction::Sin => "sin",
            BuiltinFunction::Cos => "cos",
            BuiltinFunction::Sqrt => "sqrt",
            BuiltinFunction::Exp => "exp",
            BuiltinFunction::Log => "log",
            BuiltinFunction::Rand => "rand",
        }
    }
}
