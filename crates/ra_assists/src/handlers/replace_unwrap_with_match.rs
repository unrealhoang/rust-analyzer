use std::iter;

use ra_syntax::{
    ast::{self, make},
    AstNode,
};

use crate::{Assist, AssistCtx, AssistId};
use ast::edit::IndentLevel;

// Assist: replace_unwrap_with_match
//
// Replaces `unwrap` a `match` expression. Works for Result and Option.
//
// ```
// enum Result<T, E> { Ok(T), Err(E) }
// fn main() {
//     let x: Result<i32, i32> = Result::Ok(92);
//     let y = x.<|>unwrap();
// }
// ```
// ->
// ```
// enum Result<T, E> { Ok(T), Err(E) }
// fn main() {
//     let x: Result<i32, i32> = Result::Ok(92);
//     let y = match x {
//         Ok(a) => a,
//         _ => unreachable!(),
//     };
// }
// ```
pub(crate) fn replace_unwrap_with_match(ctx: AssistCtx) -> Option<Assist> {
    let method_call: ast::MethodCallExpr = ctx.find_node_at_offset()?;
    let name = method_call.name_ref()?;
    if name.text() != "unwrap" {
        return None;
    }
    let caller = method_call.expr()?;
    let ty = ctx.sema.type_of_expr(&caller)?;

    let type_name = ty.as_adt()?.name(ctx.sema.db).to_string();

    for (unwrap_type, variant_name) in [("Result", "Ok"), ("Option", "Some")].iter() {
        if &type_name == unwrap_type {
            return ctx.add_assist(
                AssistId("replace_unwrap_with_match"),
                "Replace unwrap with match",
                |edit| {
                    let ok_path =
                        make::path_unqualified(make::path_segment(make::name_ref(variant_name)));
                    let it = make::bind_pat(make::name("a")).into();
                    let ok_tuple = make::tuple_struct_pat(ok_path, iter::once(it)).into();

                    let bind_path = make::path_unqualified(make::path_segment(make::name_ref("a")));
                    let ok_arm = make::match_arm(iter::once(ok_tuple), make::expr_path(bind_path));

                    let unreachable_call = make::unreachable_macro_call().into();
                    let err_arm = make::match_arm(
                        iter::once(make::placeholder_pat().into()),
                        unreachable_call,
                    );

                    let match_arm_list = make::match_arm_list(vec![ok_arm, err_arm]);
                    let match_expr = make::expr_match(caller.clone(), match_arm_list);
                    let match_expr =
                        IndentLevel::from_node(method_call.syntax()).increase_indent(match_expr);

                    edit.target(method_call.syntax().text_range());
                    edit.set_cursor(caller.syntax().text_range().start());
                    edit.replace_ast::<ast::Expr>(method_call.into(), match_expr);
                },
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::{check_assist, check_assist_target};

    #[test]
    fn test_replace_result_unwrap_with_match() {
        check_assist(
            replace_unwrap_with_match,
            r"
enum Result<T, E> { Ok(T), Err(E) }
fn i<T>(a: T) -> T { a }
fn main() {
    let x: Result<i32, i32> = Result::Ok(92);
    let y = i(x).<|>unwrap();
}
            ",
            r"
enum Result<T, E> { Ok(T), Err(E) }
fn i<T>(a: T) -> T { a }
fn main() {
    let x: Result<i32, i32> = Result::Ok(92);
    let y = <|>match i(x) {
        Ok(a) => a,
        _ => unreachable!(),
    };
}
            ",
        )
    }

    #[test]
    fn test_replace_option_unwrap_with_match() {
        check_assist(
            replace_unwrap_with_match,
            r"
enum Option<T> { Some(T), None }
fn i<T>(a: T) -> T { a }
fn main() {
    let x = Option::Some(92);
    let y = i(x).<|>unwrap();
}
            ",
            r"
enum Option<T> { Some(T), None }
fn i<T>(a: T) -> T { a }
fn main() {
    let x = Option::Some(92);
    let y = <|>match i(x) {
        Some(a) => a,
        _ => unreachable!(),
    };
}
            ",
        );
    }

    #[test]
    fn test_replace_result_unwrap_with_match_chaining() {
        check_assist(
            replace_unwrap_with_match,
            r"
enum Result<T, E> { Ok(T), Err(E) }
fn i<T>(a: T) -> T { a }
fn main() {
    let x: Result<i32, i32> = Result::Ok(92);
    let y = i(x).<|>unwrap().count_zeroes();
}
            ",
            r"
enum Result<T, E> { Ok(T), Err(E) }
fn i<T>(a: T) -> T { a }
fn main() {
    let x: Result<i32, i32> = Result::Ok(92);
    let y = <|>match i(x) {
        Ok(a) => a,
        _ => unreachable!(),
    }.count_zeroes();
}
            ",
        )
    }

    #[test]
    fn replace_unwrap_with_match_target() {
        check_assist_target(
            replace_unwrap_with_match,
            r"
enum Option<T> { Some(T), None }
fn i<T>(a: T) -> T { a }
fn main() {
    let x = Option::Some(92);
    let y = i(x).<|>unwrap();
}
            ",
            r"i(x).unwrap()",
        );
    }
}
