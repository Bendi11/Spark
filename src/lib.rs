use internment::LocalIntern;

pub mod parse;
pub mod util;
pub mod ast;
pub mod error;
pub mod arena;
pub mod codegen;

pub type Symbol = LocalIntern<String>;

#[cfg(test)]
mod tests {
    use std::io::Write;

    use crate::{parse::Parser, util::files::{CompiledFile, Files}, ast::DefData, ir::{lower::AstLowerer, IRContext}};

    

    const SOURCE: &str = 
r#"
type test_ty i32 | bool
type test_struct {
    i32 field
}

fun test_fn {
    let a := if true {
        phi "test\na" 
    } else {
        phi ""
    }
}

"#;
    #[test]
    pub fn test_parse() {
        let mut files = Files::new();
        let file = files.add(CompiledFile::in_memory(SOURCE.to_owned()));
        let mut parser = Parser::new(SOURCE, "buffer", file);
        let module = parser.parse().unwrap_or_else(|e| {
            for name in e.backtrace {
                eprintln!("in {}", name)
            }

            eprintln!("{}", e.error);
            if let Some(span) = e.highlighted_span {
                span.display(&files.get(file)).unwrap();
            }
            panic!()
        });

        let mut stdout = std::io::stdout();

        let mut ctx = IRContext::new();
        let mut lowerer = AstLowerer::new(&mut ctx, &files);
        lowerer.codegen(&[module]).unwrap();
        drop(lowerer);
        println!("\n{:#?}", ctx);
        panic!()
    }
}
