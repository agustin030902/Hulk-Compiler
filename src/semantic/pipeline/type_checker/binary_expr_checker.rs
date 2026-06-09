use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_binary_expr(
        &mut self,
        binary: &BinaryExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let left_type = self.check_expr(&binary.left, source);
        let right_type = self.check_expr(&binary.right, source);

        let (Some(mut left_type), Some(mut right_type)) = (left_type, right_type) else {
            return None;
        };

        match binary.op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod | BinaryOp::Pow => {
                if left_type == SemanticType::Unknown {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.left,
                        SemanticType::Number,
                        source,
                    );
                }
                if right_type == SemanticType::Unknown {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.right,
                        SemanticType::Number,
                        source,
                    );
                }

                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if left_type == SemanticType::Number && right_type == SemanticType::Number {
                    Some(SemanticType::Number)
                } else {
                    let op_name = op_symbol(binary.op.clone());
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Operator '{}' expects Number and Number, but got {} and {}.",
                            op_name,
                            left_type.display_name_with_table(&self.analyzer.type_table),
                            right_type.display_name_with_table(&self.analyzer.type_table)
                        ),
                    );
                    None
                }
            }
            BinaryOp::Concat => {
                if left_type == SemanticType::Unknown
                    && (right_type == SemanticType::Number || right_type == SemanticType::String)
                {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.left,
                        SemanticType::String,
                        source,
                    );
                }
                if right_type == SemanticType::Unknown
                    && (left_type == SemanticType::Number || left_type == SemanticType::String)
                {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.right,
                        SemanticType::String,
                        source,
                    );
                }

                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if is_valid_concat_pair(left_type, right_type) {
                    Some(SemanticType::String)
                } else {
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Operator '@' expects (String, String), (String, Number), or (Number, String), but got {} and {}.",
                            left_type.display_name_with_table(&self.analyzer.type_table),
                            right_type.display_name_with_table(&self.analyzer.type_table)
                        ),
                    );
                    None
                }
            }
            BinaryOp::ConcatSpace => {
                if left_type == SemanticType::Unknown
                    && (right_type == SemanticType::Number || right_type == SemanticType::String)
                {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.left,
                        SemanticType::String,
                        source,
                    );
                }
                if right_type == SemanticType::Unknown
                    && (left_type == SemanticType::Number || left_type == SemanticType::String)
                {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.right,
                        SemanticType::String,
                        source,
                    );
                }

                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if is_valid_concat_pair(left_type, right_type) {
                    Some(SemanticType::String)
                } else {
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Operator '@@' expects (String, String), (String, Number), or (Number, String), but got {} and {}.",
                            left_type.display_name_with_table(&self.analyzer.type_table),
                            right_type.display_name_with_table(&self.analyzer.type_table)
                        ),
                    );
                    None
                }
            }
            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
                if left_type == SemanticType::Unknown {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.left,
                        SemanticType::Number,
                        source,
                    );
                }
                if right_type == SemanticType::Unknown {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.right,
                        SemanticType::Number,
                        source,
                    );
                }

                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if left_type == SemanticType::Number && right_type == SemanticType::Number {
                    Some(SemanticType::Boolean)
                } else {
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Comparison operator '{}' expects Number and Number, but got {} and {}.",
                            op_symbol(binary.op.clone()),
                            left_type.display_name_with_table(&self.analyzer.type_table),
                            right_type.display_name_with_table(&self.analyzer.type_table)
                        ),
                    );
                    None
                }
            }
            BinaryOp::Equal | BinaryOp::NotEqual => {
                if left_type == SemanticType::Unknown && right_type != SemanticType::Unknown {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.left,
                        right_type,
                        source,
                    );
                } else if right_type == SemanticType::Unknown && left_type != SemanticType::Unknown
                {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.right,
                        left_type,
                        source,
                    );
                }

                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if left_type == SemanticType::Null || right_type == SemanticType::Null {
                    if left_type.is_nullable() && right_type.is_nullable() {
                        return Some(SemanticType::Boolean);
                    }

                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Operator '{}' compares 'Null' only with nullable operands, but got {} and {}.",
                            op_symbol(binary.op.clone()),
                            left_type.display_name_with_table(&self.analyzer.type_table),
                            right_type.display_name_with_table(&self.analyzer.type_table)
                        ),
                    );
                    return None;
                }

                if left_type != right_type {
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Operator '{}' expects operands of the same type, but got {} and {}.",
                            op_symbol(binary.op.clone()),
                            left_type.display_name_with_table(&self.analyzer.type_table),
                            right_type.display_name_with_table(&self.analyzer.type_table)
                        ),
                    );
                    return None;
                }

                if is_equality_type(left_type) {
                    Some(SemanticType::Boolean)
                } else {
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Operator '{}' only supports Number, Boolean, or String operands.",
                            op_symbol(binary.op.clone())
                        ),
                    );
                    None
                }
            }
            BinaryOp::And | BinaryOp::Or => {
                if left_type == SemanticType::Unknown {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.left,
                        SemanticType::Boolean,
                        source,
                    );
                }
                if right_type == SemanticType::Unknown {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.right,
                        SemanticType::Boolean,
                        source,
                    );
                }

                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if left_type == SemanticType::Boolean && right_type == SemanticType::Boolean {
                    Some(SemanticType::Boolean)
                } else {
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        "logical operator requires Boolean operands".to_string(),
                    );
                    None
                }
            }
        }
    }
}

fn op_symbol(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Pow => "^",
        BinaryOp::Concat => "@",
        BinaryOp::ConcatSpace => "@@",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::Greater => ">",
        BinaryOp::LessEqual => "<=",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

fn is_valid_concat_pair(left: SemanticType, right: SemanticType) -> bool {
    matches!(
        (left, right),
        (SemanticType::String, SemanticType::String)
            | (SemanticType::String, SemanticType::Number)
            | (SemanticType::Number, SemanticType::String)
    )
}

fn is_equality_type(value_type: SemanticType) -> bool {
    matches!(
        value_type,
        SemanticType::Number | SemanticType::Boolean | SemanticType::String
    )
}
