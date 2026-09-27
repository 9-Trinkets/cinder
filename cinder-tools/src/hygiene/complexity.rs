use quote::ToTokens;
use std::fs;
use std::path::{Path, PathBuf};
use syn::visit::{self, Visit};
use syn::{
    BinOp, Expr, ExprBinary, ExprForLoop, ExprIf, ExprLoop, ExprMatch, ExprWhile, File, ImplItemFn,
    Item, ItemFn, ItemImpl, ItemMod, TraitItemFn,
};

const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    "dist",
    "build",
    ".vercel",
    "data",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplexityWarning {
    pub path: PathBuf,
    pub line: usize,
    pub fn_name: String,
    pub complexity: usize,
    pub max_nesting: usize,
}

pub fn check_complexity(root: &Path, threshold: usize) -> Vec<ComplexityWarning> {
    let mut warnings = Vec::new();
    scan_dir(root, root, threshold, &mut warnings);
    warnings.sort_by(|a, b| {
        b.complexity
            .cmp(&a.complexity)
            .then_with(|| a.path.cmp(&b.path))
    });
    warnings
}

fn scan_dir(root: &Path, current: &Path, threshold: usize, out: &mut Vec<ComplexityWarning>) {
    let entries = match fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !SKIP_DIRS.contains(&name) {
                scan_dir(root, &path, threshold, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            if let Ok(content) = fs::read_to_string(&path)
                && let Ok(file) = syn::parse_file(&content)
            {
                analyze_file(&file, &rel, threshold, out);
            }
        }
    }
}

pub fn analyze_file(file: &File, path: &Path, threshold: usize, out: &mut Vec<ComplexityWarning>) {
    let mut scanner = FileScanner {
        path,
        threshold,
        current_impl_type: None,
        warnings: Vec::new(),
    };
    scanner.visit_file(file);
    out.extend(scanner.warnings);
}

struct FileScanner<'a> {
    path: &'a Path,
    threshold: usize,
    current_impl_type: Option<String>,
    warnings: Vec<ComplexityWarning>,
}

fn is_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let last_seg = attr.path().segments.last().map(|s| s.ident.to_string());
        matches!(last_seg.as_deref(), Some("test") | Some("tokio_test"))
    })
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("cfg") {
            let meta_str = attr.meta.to_token_stream().to_string();
            meta_str.contains("test")
        } else {
            false
        }
    })
}

impl<'a, 'ast> Visit<'ast> for FileScanner<'a> {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        // Skip #[cfg(test)] modules
        if is_cfg_test(&node.attrs) {
            return;
        }
        visit::visit_item_mod(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        if is_cfg_test(&node.attrs) {
            return;
        }
        let prev_impl = self.current_impl_type.take();
        self.current_impl_type = Some(node.self_ty.to_token_stream().to_string().replace(' ', ""));
        visit::visit_item_impl(self, node);
        self.current_impl_type = prev_impl;
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if is_test_attr(&node.attrs) || is_cfg_test(&node.attrs) {
            return;
        }

        let fn_name = node.sig.ident.to_string();
        let line = node.sig.ident.span().start().line;
        let mut visitor = FnComplexityVisitor::new();
        visitor.visit_block(&node.block);

        if visitor.complexity > self.threshold {
            self.warnings.push(ComplexityWarning {
                path: self.path.to_path_buf(),
                line,
                fn_name,
                complexity: visitor.complexity,
                max_nesting: visitor.max_nesting,
            });
        }

        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        if is_test_attr(&node.attrs) || is_cfg_test(&node.attrs) {
            return;
        }

        let name = node.sig.ident.to_string();
        let fn_name = if let Some(ref ty) = self.current_impl_type {
            format!("{ty}::{name}")
        } else {
            name
        };
        let line = node.sig.ident.span().start().line;
        let mut visitor = FnComplexityVisitor::new();
        visitor.visit_block(&node.block);

        if visitor.complexity > self.threshold {
            self.warnings.push(ComplexityWarning {
                path: self.path.to_path_buf(),
                line,
                fn_name,
                complexity: visitor.complexity,
                max_nesting: visitor.max_nesting,
            });
        }

        visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        if let Some(ref default_block) = node.default {
            let fn_name = node.sig.ident.to_string();
            let line = node.sig.ident.span().start().line;
            let mut visitor = FnComplexityVisitor::new();
            visitor.visit_block(default_block);

            if visitor.complexity > self.threshold {
                self.warnings.push(ComplexityWarning {
                    path: self.path.to_path_buf(),
                    line,
                    fn_name,
                    complexity: visitor.complexity,
                    max_nesting: visitor.max_nesting,
                });
            }
        }
        visit::visit_trait_item_fn(self, node);
    }
}

pub struct FnComplexityVisitor {
    pub complexity: usize,
    pub current_nesting: usize,
    pub max_nesting: usize,
}

impl FnComplexityVisitor {
    pub fn new() -> Self {
        Self {
            complexity: 1, // Base McCabe complexity is 1
            current_nesting: 0,
            max_nesting: 0,
        }
    }

    fn enter_block<F: FnOnce(&mut Self)>(&mut self, f: F) {
        self.current_nesting += 1;
        if self.current_nesting > self.max_nesting {
            self.max_nesting = self.current_nesting;
        }
        f(self);
        self.current_nesting -= 1;
    }
}

impl<'ast> Visit<'ast> for FnComplexityVisitor {
    fn visit_expr_if(&mut self, node: &'ast ExprIf) {
        self.complexity += 1;
        self.visit_expr(&node.cond);

        self.enter_block(|v| {
            for stmt in &node.then_branch.stmts {
                v.visit_stmt(stmt);
            }
        });

        if let Some((_, ref else_expr)) = node.else_branch {
            match &**else_expr {
                // Flat `else if` does not increment nesting depth
                Expr::If(else_if) => {
                    self.visit_expr_if(else_if);
                }
                other => {
                    self.enter_block(|v| {
                        v.visit_expr(other);
                    });
                }
            }
        }
    }

    fn visit_expr_while(&mut self, node: &'ast ExprWhile) {
        self.complexity += 1;
        self.visit_expr(&node.cond);
        self.enter_block(|v| {
            for stmt in &node.body.stmts {
                v.visit_stmt(stmt);
            }
        });
    }

    fn visit_expr_for_loop(&mut self, node: &'ast ExprForLoop) {
        self.complexity += 1;
        self.visit_expr(&node.expr);
        self.enter_block(|v| {
            for stmt in &node.body.stmts {
                v.visit_stmt(stmt);
            }
        });
    }

    fn visit_expr_loop(&mut self, node: &'ast ExprLoop) {
        self.complexity += 1;
        self.enter_block(|v| {
            for stmt in &node.body.stmts {
                v.visit_stmt(stmt);
            }
        });
    }

    fn visit_expr_match(&mut self, node: &'ast ExprMatch) {
        let arm_count = node.arms.len();
        if arm_count > 1 {
            self.complexity += arm_count - 1;
        }

        self.visit_expr(&node.expr);

        for arm in &node.arms {
            if let Some((_, ref guard_expr)) = arm.guard {
                self.complexity += 1;
                self.visit_expr(guard_expr);
            }
            self.enter_block(|v| {
                v.visit_expr(&arm.body);
            });
        }
    }

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        if matches!(node.op, BinOp::And(_) | BinOp::Or(_)) {
            self.complexity += 1;
        }
        visit::visit_expr_binary(self, node);
    }

    fn visit_item(&mut self, _node: &'ast Item) {
        // Do not traverse inner items (e.g. nested fn) in the same function metrics;
        // FileScanner will visit inner items as separate functions.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_function_has_complexity_one() {
        let code = r#"
            fn simple() {
                let a = 1;
                let b = 2;
                println!("{}", a + b);
            }
        "#;
        let file = syn::parse_str(code).unwrap();
        let mut out = Vec::new();
        analyze_file(&file, Path::new("test.rs"), 0, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].complexity, 1);
        assert_eq!(out[0].max_nesting, 0);
    }

    #[test]
    fn if_else_adds_complexity() {
        let code = r#"
            fn branch(x: i32) {
                if x > 0 {
                    println!("pos");
                } else {
                    println!("non-pos");
                }
            }
        "#;
        let file = syn::parse_str(code).unwrap();
        let mut out = Vec::new();
        analyze_file(&file, Path::new("test.rs"), 0, &mut out);
        assert_eq!(out[0].complexity, 2);
        assert_eq!(out[0].max_nesting, 1);
    }

    #[test]
    fn flat_else_if_increments_complexity_without_increasing_nesting() {
        let code = r#"
            fn check_color(c: &str) {
                if c == "red" {
                    println!("red");
                } else if c == "green" {
                    println!("green");
                } else if c == "blue" {
                    println!("blue");
                } else {
                    println!("other");
                }
            }
        "#;
        let file = syn::parse_str(code).unwrap();
        let mut out = Vec::new();
        analyze_file(&file, Path::new("test.rs"), 0, &mut out);
        assert_eq!(out[0].complexity, 4); // base 1 + 3 ifs
        assert_eq!(out[0].max_nesting, 1);
    }

    #[test]
    fn nested_ifs_increase_nesting_depth() {
        let code = r#"
            fn deeply_nested(a: bool, b: bool, c: bool) {
                if a {
                    if b {
                        if c {
                            println!("deep");
                        }
                    }
                }
            }
        "#;
        let file = syn::parse_str(code).unwrap();
        let mut out = Vec::new();
        analyze_file(&file, Path::new("test.rs"), 0, &mut out);
        assert_eq!(out[0].complexity, 4);
        assert_eq!(out[0].max_nesting, 3);
    }

    #[test]
    fn match_with_arms_and_guards() {
        let code = r#"
            fn evaluate(val: Option<i32>) {
                match val {
                    Some(x) if x > 10 => println!("large"),
                    Some(_) => println!("other"),
                    None => println!("none"),
                }
            }
        "#;
        let file = syn::parse_str(code).unwrap();
        let mut out = Vec::new();
        analyze_file(&file, Path::new("test.rs"), 0, &mut out);
        // Base: 1
        // Match 3 arms: +2
        // Guard (if x > 10): +1
        // Total = 4
        assert_eq!(out[0].complexity, 4);
        assert_eq!(out[0].max_nesting, 1);
    }

    #[test]
    fn boolean_operators_increase_complexity() {
        let code = r#"
            fn bool_logic(a: bool, b: bool, c: bool) {
                if a && (b || c) {
                    println!("ok");
                }
            }
        "#;
        let file = syn::parse_str(code).unwrap();
        let mut out = Vec::new();
        analyze_file(&file, Path::new("test.rs"), 0, &mut out);
        // Base 1 + if (1) + && (1) + || (1) = 4
        assert_eq!(out[0].complexity, 4);
    }

    #[test]
    fn impl_method_name_includes_type() {
        let code = r#"
            struct Foo;
            impl Foo {
                fn bar(&self, x: bool) {
                    if x {
                        println!("yes");
                    }
                }
            }
        "#;
        let file = syn::parse_str(code).unwrap();
        let mut out = Vec::new();
        analyze_file(&file, Path::new("test.rs"), 0, &mut out);
        assert_eq!(out[0].fn_name, "Foo::bar");
        assert_eq!(out[0].complexity, 2);
    }

    #[test]
    fn test_functions_are_skipped() {
        let code = r#"
            #[test]
            fn test_something() {
                if true {
                    if true {
                        assert!(true);
                    }
                }
            }
        "#;
        let file = syn::parse_str(code).unwrap();
        let mut out = Vec::new();
        analyze_file(&file, Path::new("test.rs"), 0, &mut out);
        assert!(out.is_empty());
    }
}
