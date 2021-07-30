use std::convert::TryFrom;

use inkwell::{InlineAsmDialect, OptimizationLevel, module::Module, passes::PassManager, targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine}, values::{BasicValue, CallableValue}};
use log::warn;

use crate::{
    ast::{Ast, AstPos, FunProto},
    code::linker::Linker,
    lex::Pos,
    CompileOpts, OutFormat,
};

use super::{debug, error, Compiler};

impl<'a, 'c> Compiler<'a, 'c> {
    /// Generate code for a full function definition
    pub fn gen_fundef(&mut self, proto: &FunProto, body: &[AstPos], pos: &Pos) -> Option<()> {
        if self.current_fn.is_some() {
            error!("{}: Nested functions are not currently supported, function {} must be moved to the top level", pos, proto.name);
            return None;
        }

        let old_vars = self.vars.clone();

        let f = match self.module.get_function(
            self.current_ns
                .get()
                .qualify(&proto.name)
                .to_string()
                .as_str(),
        ) {
            Some(f) => f,
            None => self.gen_fun_proto(proto, pos).unwrap(),
        };
        self.current_fn = Some(f);
        self.current_proto = Some(proto.clone());

        let bb = self.ctx.append_basic_block(f, "fn_entry"); //Add the first basic block
        self.build.position_at_end(bb); //Start inserting into the function

        //Add argument names to the list of variables we can use
        for (arg, (ty, proto_arg)) in f.get_param_iter().zip(proto.args.iter()) {
            let alloca = self.entry_alloca(
                proto_arg.clone().unwrap_or_else(String::new).as_str(),
                self.llvm_type(ty, pos),
            );
            self.build.build_store(alloca, arg); //Store the initial value in the function parameters

            if let Some(name) = proto_arg {
                self.vars.insert(name.clone(), (alloca, ty.clone()));
            }
        }

        //Generate code for the function body
        for ast in body {
            match self.gen(ast, false) {
                Some(_) => (),
                None => {
                    debug!(
                        "Not generating more code for function {} due to fatal error",
                        proto.name
                    );
                    break;
                }
            }
        }

        self.vars = old_vars; //Reset the variables
        self.current_fn = None;
        self.current_proto = None;
        Some(())
    }

    /// Generate an assembly function definition
    fn gen_asmfundef(&mut self, proto: FunProto, asm: String, constraints: String, pos: Pos) -> Option<()> {
        if self.current_fn.is_some() {
            error!("Nested functions are not currently supported, function {} must be moved to the top level", proto.name);
            return None;
        }
        
        let fun = match self.module.get_function(
            self.current_ns
                .get()
                .qualify(&proto.name)
                .to_string()
                .as_str(),
        ) {
            Some(f) => f,
            None => self.gen_fun_proto(&proto, &pos).unwrap(),
        };

        let bb = self.ctx.append_basic_block(fun, "asm_fn_entry"); //Add the first basic block
        self.build.position_at_end(bb); //Start inserting into the function

        //Small hack: Inline asm creates a function pointer, so we create a function pointer and call it in the function body

        let asm = self.ctx.create_inline_asm(fun.get_type(), asm, constraints, true, false, Some(InlineAsmDialect::Intel));
        let params = fun.get_params();
        let callable_asm = CallableValue::try_from(asm).unwrap();
        
        let call = self.build.build_call(callable_asm, &params, "asm_fn_call_asm")
            .try_as_basic_value()
            .left();
        let call = call.as_ref().map(|c| c as &dyn BasicValue);
        self.build.build_return(call);
        self.current_fn = None;
        self.current_proto = None;
        Some(())
    }

    /// Generate top level expressions in an AST
    fn gen_top(&mut self, ast: Vec<AstPos>) -> Result<(), u16> {
        let mut err = 0;
        for node in ast {
            match node.0 {
                Ast::FunDef(ref proto, ref body) => {
                    if self.gen_fundef(proto, body, &node.1).is_none() {
                        err += 1
                    }
                },
                Ast::AsmFunDef(proto, asm, constraints) => {
                    if self.gen_asmfundef(proto, asm, constraints, node.1).is_none() {
                        err += 1
                    }
                },
                Ast::Ns(ref path, stmts) => {
                    self.enter_ns(path);
                    self.gen_top(stmts)?;
                    self.exit_ns(path.count());
                }
                other => {
                    error!("{}: Invalid top level expression {:?}", node.1, other);
                    err += 1;
                }
            }
        }
        match err > 0 {
            true => Err(err),
            false => Ok(()),
        }
    }

    /// Generate all code for a LLVM module and return it
    pub fn finish(mut self, ast: Vec<AstPos>) -> Result<Module<'c>, u16> {
        let ast = self.scan_decls(ast);
        let ast = self.get_consts(ast);
        self.gen_top(ast)?;
        Ok(self.module)
    }

    /// Compile the code into an executable / library file
    pub fn compile<L: Linker>(
        self,
        ast: Vec<AstPos>,
        opts: CompileOpts,
        mut linker: L,
    ) -> Result<(), u16> {
        let module = self.finish(ast)?;

        let res = module.verify();

        match opts.ignore_checks {
            true => {
                if let Err(e) = res {
                    warn!(
                        "LLVM Module verification (ignored due to command line switch): {}",
                        e
                    )
                }
            }
            false => {
                if let Err(e) = res {
                    error!("LLVM Module verification failed: {}", e);
                    return Err(1);
                }
            }
        }

        let fpm: PassManager<Module<'c>> = PassManager::create(());

        match opts.opt_lvl {
            crate::OptLvl::Debug => (),
            crate::OptLvl::Medium => {
                fpm.add_demote_memory_to_register_pass();
                fpm.add_promote_memory_to_register_pass();
                fpm.add_constant_merge_pass();
                fpm.add_instruction_combining_pass();
                fpm.add_global_optimizer_pass();
            }
            crate::OptLvl::Aggressive => {
                fpm.add_demote_memory_to_register_pass();
                fpm.add_promote_memory_to_register_pass();
                fpm.add_constant_merge_pass();
                fpm.add_instruction_combining_pass();
                fpm.add_global_optimizer_pass();

                fpm.add_loop_rotate_pass();
                fpm.add_argument_promotion_pass();
                fpm.add_function_inlining_pass();
                fpm.add_memcpy_optimize_pass();
                fpm.add_loop_deletion_pass();
                fpm.add_loop_vectorize_pass();
                fpm.add_constant_propagation_pass();
                fpm.add_simplify_lib_calls_pass();
                fpm.add_strip_symbol_pass();
            }
        }

        match opts.output_ty {
            OutFormat::IR => module.print_to_file(opts.out_file.with_extension("ll")).unwrap(),
            other => {
                Target::initialize_all(&InitializationConfig::default());
                let opt = OptimizationLevel::Aggressive;
                let reloc = RelocMode::Default;
                let model = CodeModel::Default;
                let target = Target::from_triple(&TargetMachine::get_default_triple()).unwrap();
                let machine = target
                    .create_target_machine(
                        &TargetMachine::get_default_triple(),
                        TargetMachine::get_host_cpu_name().to_str().unwrap(),
                        TargetMachine::get_host_cpu_features().to_str().unwrap(),
                        opt,
                        reloc,
                        model,
                    )
                    .unwrap();

                machine.add_analysis_passes(&fpm);

                match other {
                    OutFormat::Asm => machine
                        .write_to_file(&module, FileType::Assembly, &opts.out_file.with_extension("s"))
                        .unwrap(),
                    OutFormat::Obj => machine
                        .write_to_file(&module, FileType::Object, &opts.out_file.with_extension("o"))
                        .unwrap(),
                    OutFormat::Lib => {
                        let obj = opts.out_file.with_extension("obj");
                        machine
                            .write_to_file(&module, FileType::Object, &obj)
                            .unwrap();

                        for lib in opts.libraries {
                            linker.add_library(lib);
                        }

                        linker.add_object_file(obj.to_str().unwrap().to_owned());
                        linker.set_format(OutFormat::Lib);
                        //linker.set_entry(Some("main"));
                        linker.set_output_file(opts.out_file);
                        linker.link().unwrap();

                        std::fs::remove_file(obj).unwrap();
                    }
                    OutFormat::Exe => {
                        let obj = opts.out_file.with_extension("obj");
                        machine
                            .write_to_file(&module, FileType::Object, &obj)
                            .unwrap();

                        for lib in opts.libraries {
                            linker.add_library(lib);
                        }

                        linker.add_object_file(obj.to_str().unwrap().to_owned());
                        linker.set_format(OutFormat::Exe);
                        linker.set_entry(Some("main"));
                        linker.set_output_file(opts.out_file);
                        linker.link().unwrap();

                        std::fs::remove_file(obj).unwrap();
                    }
                    _ => unreachable!(),
                }
            }
        }

        Ok(())
    }
}
