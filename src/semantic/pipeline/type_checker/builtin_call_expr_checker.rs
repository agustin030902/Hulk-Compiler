use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_print_argument(
        &mut self,
        arg: &Expr,
        span: Span,
        source: &str,
    ) -> Option<SemanticType> {
        let arg_type = self.check_expr(arg, source)?;

        if arg_type == SemanticType::Unknown {
            return Some(SemanticType::Unknown);
        }

        if arg_type == SemanticType::Unit {
            self.analyzer.push_type_error(
                span,
                source,
                "Function 'print' expects a non-Unit argument, but got Unit.".to_string(),
            );
            return None;
        }

        if arg_type == SemanticType::Null {
            self.analyzer.push_type_error(
                span,
                source,
                "Function 'print' cannot print values of type Null.".to_string(),
            );
            return None;
        }

        if matches!(
            arg_type,
            SemanticType::Function(_) | SemanticType::Struct(_)
        ) {
            self.analyzer.push_type_error(
                span,
                source,
                format!(
                    "Function 'print' cannot print values of type {}.",
                    arg_type.display_name()
                ),
            );
            return None;
        }

        Some(SemanticType::Unit)
    }

    pub(super) fn check_builtin_call(
        &mut self,
        function: BuiltinFunction,
        args: &[Expr],
        span: Span,
        source: &str,
    ) -> Option<SemanticType> {
        match function {
            BuiltinFunction::Print => {
                if args.len() != 1 {
                    self.analyzer.push_semantic_error(
                        span,
                        source,
                        "Function 'print' expects 1 argument.".to_string(),
                    );
                    return None;
                }
                let arg = &args[0];
                self.check_print_argument(arg, span, source)
            }
            BuiltinFunction::Sin
            | BuiltinFunction::Cos
            | BuiltinFunction::Sqrt
            | BuiltinFunction::Exp => {
                if args.len() != 1 {
                    self.analyzer.push_semantic_error(
                        span,
                        source,
                        format!("Function '{}' expects 1 argument.", function.name()),
                    );
                    return None;
                }
                let arg = &args[0];

                let mut arg_type = self.check_expr(arg, source)?;
                if arg_type == SemanticType::Unknown {
                    arg_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        arg,
                        SemanticType::Number,
                        source,
                    );
                }
                if arg_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if arg_type == SemanticType::Number {
                    Some(SemanticType::Number)
                } else {
                    self.analyzer.push_type_error(
                        span,
                        source,
                        format!(
                            "Function '{}' expects Number, but got {}.",
                            function.name(),
                            arg_type.display_name()
                        ),
                    );
                    None
                }
            }
            BuiltinFunction::Log => {
                if args.len() != 2 {
                    self.analyzer.push_semantic_error(
                        span,
                        source,
                        "Function 'log' expects 2 arguments.".to_string(),
                    );
                    return None;
                }

                let mut left_type = self.check_expr(&args[0], source)?;
                let mut right_type = self.check_expr(&args[1], source)?;
                if left_type == SemanticType::Unknown {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &args[0],
                        SemanticType::Number,
                        source,
                    );
                }
                if right_type == SemanticType::Unknown {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &args[1],
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
                    self.analyzer.push_type_error(
                        span,
                        source,
                        format!(
                            "Function 'log' expects (Number, Number), but got {} and {}.",
                            left_type.display_name(),
                            right_type.display_name()
                        ),
                    );
                    None
                }
            }
            BuiltinFunction::Rand => {
                if !args.is_empty() {
                    self.analyzer.push_semantic_error(
                        span,
                        source,
                        "Function 'rand' expects 0 arguments.".to_string(),
                    );
                    return None;
                }

                Some(SemanticType::Number)
            }
        }
    }
}
