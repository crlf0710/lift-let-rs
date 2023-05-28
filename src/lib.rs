use proc_macro::TokenStream;
use quote::ToTokens;
use syn::spanned::Spanned;

struct GenVarName(usize, proc_macro::Span);

impl GenVarName {
    fn next_var_name(&mut self) -> syn::Ident {
        let result = syn::Ident::new(&format!("__lift_let_{:04}", self.0), self.1.into());
        self.0 += 1;
        result
    }
}

struct BlockRecord {
    cur_statement_idx: isize,
    original_statement_count: usize,
    statements_to_be_inserted: Vec<(usize, syn::Stmt)>,
}

struct LiftLet {
    gen_var_name: GenVarName,
    block_records: Vec<BlockRecord>,
    error: Option<syn::Error>,
}

fn is_let_attribute(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("let")
}

impl syn::visit_mut::VisitMut for LiftLet {
    fn visit_attribute_mut(&mut self, i: &mut syn::Attribute) {
        if self.error.is_some() {
            return;
        }
        if is_let_attribute(i) {
            self.error = Some(syn::Error::new(
                i.span(),
                "`#[let]` used in a non-recognizable way",
            ));
            return;
        }
        syn::visit_mut::visit_attribute_mut(self, i);
    }

    fn visit_block_mut(&mut self, i: &mut syn::Block) {
        if self.error.is_some() {
            return;
        }
        self.block_records.push(BlockRecord {
            cur_statement_idx: -1,
            original_statement_count: i.stmts.len(),
            statements_to_be_inserted: Vec::new(),
        });
        syn::visit_mut::visit_block_mut(self, i);
        let block_record = self
            .block_records
            .pop()
            .expect("Syntax tree visiting events didn't match");
        assert!((block_record.cur_statement_idx + 1) as usize == i.stmts.len());
        for (stmt_idx, stmt_to_be_inserted) in
            block_record.statements_to_be_inserted.into_iter().rev()
        {
            i.stmts.insert(stmt_idx, stmt_to_be_inserted);
        }
    }

    fn visit_stmt_mut(&mut self, i: &mut syn::Stmt) {
        let block_record = self
            .block_records
            .last_mut()
            .expect("Syntax tree visiting events didn't match");
        block_record.cur_statement_idx += 1;
        assert!((block_record.cur_statement_idx as usize) < block_record.original_statement_count);
        if self.error.is_some() {
            return;
        }
        syn::visit_mut::visit_stmt_mut(self, i);
    }

    fn visit_expr_paren_mut(&mut self, i: &mut syn::ExprParen) {
        if self.error.is_some() {
            return;
        }
        let mut has_let = false;
        i.attrs.retain_mut(|attr| {
            if is_let_attribute(attr) {
                has_let = true;
                false
            } else {
                true
            }
        });
        if has_let {
            let block_record = self
                .block_records
                .last_mut()
                .expect("Syntax tree visiting events didn't match");
            let var_ident = self.gen_var_name.next_var_name();
            let expr_var = syn::Expr::Path(syn::ExprPath {
                path: var_ident.clone().into(),
                attrs: Default::default(),
                qself: Default::default(),
            });
            let original_expr = std::mem::replace(&mut i.expr, Box::new(expr_var));
            let let_stmt = syn::Stmt::Local(syn::Local {
                pat: syn::Pat::Ident(syn::PatIdent {
                    ident: var_ident,
                    subpat: Default::default(),
                    attrs: Default::default(),
                    by_ref: Default::default(),
                    mutability: Default::default(),
                }),
                init: Some(syn::LocalInit {
                    expr: original_expr,
                    eq_token: Default::default(),
                    diverge: Default::default(),
                }),
                attrs: Default::default(),
                semi_token: Default::default(),
                let_token: Default::default(),
            });
            assert!(
                block_record.cur_statement_idx >= 0
                    && (block_record.cur_statement_idx as usize)
                        < block_record.original_statement_count
            );
            block_record
                .statements_to_be_inserted
                .push((block_record.cur_statement_idx as usize, let_stmt));
        }
        syn::visit_mut::visit_expr_paren_mut(self, i);
    }

    fn visit_expr_mut(&mut self, i: &mut syn::Expr) {
        // This is strictly unnecessary, however it can give better diagnostics.
        if self.error.is_some() {
            return;
        }
        match i {
            syn::Expr::Paren(..) => {}
            syn::Expr::Verbatim(..) => {}
            syn::Expr::Array(syn::ExprArray { attrs, .. })
            | syn::Expr::Assign(syn::ExprAssign { attrs, .. })
            | syn::Expr::Async(syn::ExprAsync { attrs, .. })
            | syn::Expr::Await(syn::ExprAwait { attrs, .. })
            | syn::Expr::Binary(syn::ExprBinary { attrs, .. })
            | syn::Expr::Block(syn::ExprBlock { attrs, .. })
            | syn::Expr::Break(syn::ExprBreak { attrs, .. })
            | syn::Expr::Call(syn::ExprCall { attrs, .. })
            | syn::Expr::Cast(syn::ExprCast { attrs, .. })
            | syn::Expr::Closure(syn::ExprClosure { attrs, .. })
            | syn::Expr::Const(syn::ExprConst { attrs, .. })
            | syn::Expr::Continue(syn::ExprContinue { attrs, .. })
            | syn::Expr::Field(syn::ExprField { attrs, .. })
            | syn::Expr::ForLoop(syn::ExprForLoop { attrs, .. })
            | syn::Expr::Group(syn::ExprGroup { attrs, .. })
            | syn::Expr::If(syn::ExprIf { attrs, .. })
            | syn::Expr::Index(syn::ExprIndex { attrs, .. })
            | syn::Expr::Infer(syn::ExprInfer { attrs, .. })
            | syn::Expr::Let(syn::ExprLet { attrs, .. })
            | syn::Expr::Lit(syn::ExprLit { attrs, .. })
            | syn::Expr::Loop(syn::ExprLoop { attrs, .. })
            | syn::Expr::Macro(syn::ExprMacro { attrs, .. })
            | syn::Expr::Match(syn::ExprMatch { attrs, .. })
            | syn::Expr::MethodCall(syn::ExprMethodCall { attrs, .. })
            | syn::Expr::Path(syn::ExprPath { attrs, .. })
            | syn::Expr::Range(syn::ExprRange { attrs, .. })
            | syn::Expr::Reference(syn::ExprReference { attrs, .. })
            | syn::Expr::Repeat(syn::ExprRepeat { attrs, .. })
            | syn::Expr::Return(syn::ExprReturn { attrs, .. })
            | syn::Expr::Struct(syn::ExprStruct { attrs, .. })
            | syn::Expr::Try(syn::ExprTry { attrs, .. })
            | syn::Expr::TryBlock(syn::ExprTryBlock { attrs, .. })
            | syn::Expr::Tuple(syn::ExprTuple { attrs, .. })
            | syn::Expr::Unary(syn::ExprUnary { attrs, .. })
            | syn::Expr::Unsafe(syn::ExprUnsafe { attrs, .. })
            | syn::Expr::While(syn::ExprWhile { attrs, .. })
            | syn::Expr::Yield(syn::ExprYield { attrs, .. }) => {
                if attrs.iter().any(is_let_attribute) {
                    self.error = Some(syn::Error::new(
                        i.span(),
                        "`#[let]` used in an expression that is not parenthesized",
                    ));
                    return;
                }
            }
            _ => {
                self.error = Some(syn::Error::new(i.span(), "new expression type encountered"));
                return;
            }
        }
        syn::visit_mut::visit_expr_mut(self, i);
    }
}

#[proc_macro_attribute]
pub fn lift_let(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(item as syn::Item);
    let gen_var_name = GenVarName(0, proc_macro::Span::call_site());
    let mut lift_let = LiftLet {
        gen_var_name,
        block_records: Vec::new(),
        error: None,
    };
    syn::visit_mut::visit_item_mut(&mut lift_let, &mut item);
    if let Some(err) = lift_let.error {
        return err.to_compile_error().into();
    }
    item.to_token_stream().into()
}
