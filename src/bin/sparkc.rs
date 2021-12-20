

fn main() {
   test_parse() 
}

use std::io::Write;

    use string_interner::StringInterner;

    use spark::{parse::Parser, util::files::CompiledFile, ast::DefData};

    

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
    let b := {
        let testing := a[0]
        phi testing
    }:(132)

    let(fun()->()) a := {}
}

"#;
    
    pub fn test_parse() {
        let file = CompiledFile::in_memory(SOURCE.to_owned());
        let mut interner = StringInterner::new();
        let mut parser = Parser::new(SOURCE, &mut interner, "buffer");
        let module = parser.parse().unwrap_or_else(|e| {
            for name in e.backtrace {
                eprintln!("in {}", name)
            }

            eprintln!("{}", e.error);
            if let Some(span) = e.highlighted_span {
                span.display(&file).unwrap();
            }
            panic!()
        });

        let mut stdout = std::io::stdout();
        for (_, def) in module.defs {
            if let DefData::FunDef(_, body) = def.data {
                for expr in body {
                    expr.node.display(&mut stdout, &interner, 0).unwrap();
                    writeln!(&mut stdout).unwrap();
                }
            }
        }
        
    }
