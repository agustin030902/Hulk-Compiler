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
    pub types: Vec<TypeDecl>,
    pub interfaces: Vec<InterfaceDecl>,
    pub functions: Vec<FunctionDecl>,
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub enum ProgramItem {
    Interface(InterfaceDecl),
    Type(TypeDecl),
    Function(FunctionDecl),
}

#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub name: String,
    pub name_span: Span,
    pub params: Vec<TypeParam>,
    pub parent_name: Option<String>,
    pub parent_span: Option<Span>,
    pub parent_init_exprs: Vec<Expr>,
    pub attributes: Vec<TypeAttribute>,
    pub methods: Vec<MethodDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeParam {
    pub name: String,
    pub type_annotation: Option<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeAttribute {
    pub name: String,
    pub name_span: Span,
    pub type_annotation: Option<TypeAnnotation>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodDecl {
    pub name: String,
    pub name_span: Span,
    pub params: Vec<FunctionParam>,
    pub return_type_annotation: Option<TypeAnnotation>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceDecl {
    pub name: String,
    pub name_span: Span,
    pub parent_name: Option<String>,
    pub parent_span: Option<Span>,
    pub methods: Vec<InterfaceMethodDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceMethodDecl {
    pub name: String,
    pub name_span: Span,
    pub params: Vec<FunctionParam>,
    pub return_type_annotation: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeMemberDecl {
    Attribute(TypeAttribute),
    Method(MethodDecl),
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub name_span: Span,
    pub params: Vec<FunctionParam>,
    pub return_type_annotation: Option<TypeAnnotation>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionParam {
    pub name: String,
    pub type_annotation: Option<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeAnnotation {
    pub name: String,
    pub span: Span,
    pub is_splat: bool,
    pub is_vector: bool,
    pub vector_dims: usize,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Let {
        name: String,
        name_span: Span,
        type_annotation: Option<TypeAnnotation>,
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
    FunctionCall(FunctionCallExpr),
    MethodCall(MethodCallExpr),
    MemberAccess(MemberAccessExpr),
    New(NewExpr),
    NewArray(NewArrayExpr),
    ArrayIndex(ArrayIndexExpr),
    ArrayLiteral(ArrayLiteralExpr),
    DestructiveAssign(DestructiveAssignExpr),
    LetIn(LetInExpr),
    Block(BlockExpr),
    While(WhileExpr),
    For(ForExpr),
    If(IfExpr),
    Is(IsExpr),
    As(AsExpr),
    Literal { value: Literal, span: Span },
    Variable { name: String, span: Span },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Binary(binary) => binary.span,
            Expr::Unary(unary) => unary.span,
            Expr::BuiltinCall(call) => call.span,
            Expr::FunctionCall(call) => call.span,
            Expr::MethodCall(call) => call.span,
            Expr::MemberAccess(access) => access.span,
            Expr::New(new_expr) => new_expr.span,
            Expr::NewArray(new_array) => new_array.span,
            Expr::ArrayIndex(array_index) => array_index.span,
            Expr::ArrayLiteral(array_literal) => array_literal.span,
            Expr::DestructiveAssign(assign) => assign.span,
            Expr::LetIn(let_in) => let_in.span,
            Expr::Block(block) => block.span,
            Expr::While(while_expr) => while_expr.span,
            Expr::For(for_expr) => for_expr.span,
            Expr::If(if_expr) => if_expr.span,
            Expr::Is(is_expr) => is_expr.span,
            Expr::As(as_expr) => as_expr.span,
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
    Null,
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
    pub target: AssignTarget,
    pub value: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    Variable {
        name: String,
        name_span: Span,
    },
    Member {
        object: Box<Expr>,
        member: String,
        member_span: Span,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
}

impl AssignTarget {
    pub fn span(&self) -> Span {
        match self {
            AssignTarget::Variable { name_span, .. } => *name_span,
            AssignTarget::Member { span, .. } => *span,
            AssignTarget::Index { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuiltinCallExpr {
    pub function: BuiltinFunction,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionCallExpr {
    pub name: String,
    pub name_span: Span,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodCallExpr {
    pub receiver: Box<Expr>,
    pub method_name: String,
    pub method_name_span: Span,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MemberAccessExpr {
    pub object: Box<Expr>,
    pub member: String,
    pub member_span: Span,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct NewExpr {
    pub type_name: String,
    pub type_name_span: Span,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct NewArrayExpr {
    pub type_name: String,
    pub type_name_span: Span,
    pub sizes: Vec<Expr>,
    pub initializer: Option<LambdaExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ArrayIndexExpr {
    pub object: Box<Expr>,
    pub index: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ArrayLiteralExpr {
    pub elements: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LambdaExpr {
    pub param_name: String,
    pub param_span: Span,
    pub body: Box<Expr>,
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
pub struct WhileExpr {
    pub condition: Box<Expr>,
    pub body: BlockExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForExpr {
    pub id: String,
    pub id_span: Span,
    pub iter: Box<Expr>,
    pub body: BlockExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Box<Expr>,
    pub then_branch: Box<Expr>,
    pub elif_branches: Vec<ElifBranch>,
    pub else_branch: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IsExpr {
    pub expr: Box<Expr>,
    pub target_type: String,
    pub target_type_span: Span,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AsExpr {
    pub expr: Box<Expr>,
    pub target_type: String,
    pub target_type_span: Span,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ElifBranch {
    pub condition: Expr,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub type_annotation: Option<TypeAnnotation>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Pow,
    Concat,
    ConcatSpace,
    Sub,
    Mul,
    Div,
    Mod,
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
