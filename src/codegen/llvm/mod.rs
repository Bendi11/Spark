//! Generating LLVM IR from a parsed and type lowered AST

pub mod astgen;

use std::convert::TryFrom;

use hashbrown::HashMap;
use inkwell::{
    AddressSpace, 
    builder::Builder,
    context::Context,
    module::{Module, Linkage},
    targets::{TargetData, TargetMachine, Target, InitializationConfig, RelocMode, CodeModel},
    types::{AnyTypeEnum, BasicType, BasicTypeEnum, FunctionType as InkwellFunctionType},
    values::{FunctionValue, BasicValueEnum}, OptimizationLevel
};
use quickscope::ScopeMap;

use crate::{
    codegen::ir::{ModId, SparkCtx, TypeId, SparkDef, TypeData, FunctionType, FunId},
    ast::IntegerWidth, error::DiagnosticManager, util::files::Files, Symbol,
};

/// A type representing all types that can be defined in the global scope 
/// map of the code generator
enum ScopeDef<'ctx> {
    Value(BasicValueEnum<'ctx>),
    Def(SparkDef),
}

/// Structure that generates LLVM IR modules from a parsed and 
/// type lowered AST module
pub struct LlvmCodeGenerator<'ctx, 'files> {
    pub ctx: &'ctx Context,
    pub builder: Builder<'ctx>,
    pub spark: SparkCtx,
    pub diags: DiagnosticManager<'files>,
    llvm_funs: HashMap<FunId, FunctionValue<'ctx>>,
    target: TargetData,
    current_scope: ScopeMap<Symbol, ScopeDef<'ctx>>,
}

impl<'ctx, 'files> LlvmCodeGenerator<'ctx, 'files> {
    /// Create a new code generator from an LLVM context
    pub fn new(spark: SparkCtx, ctx: &'ctx Context, files: &'files Files) -> Self {
        Target::initialize_native(&InitializationConfig::default()).expect("LLVM: failed to initialize native compilation target");

        Self {
            current_scope: ScopeMap::new(),
            builder: ctx.create_builder(),
            ctx,
            spark,
            diags: DiagnosticManager::new(files),
            llvm_funs: HashMap::new(),
            target: Target::from_triple(&TargetMachine::get_default_triple()).unwrap().create_target_machine(
                &TargetMachine::get_default_triple(),
                TargetMachine::get_host_cpu_name().to_str().unwrap(),
                TargetMachine::get_host_cpu_features().to_str().unwrap(),
                OptimizationLevel::Default,
                RelocMode::Default,
                CodeModel::Medium
            ).unwrap().get_target_data(),
        }
    }
    
    /// Codegen LLVM IR from a type-lowered module
    pub fn codegen_module(&mut self, module: ModId) -> Module<'ctx> {
        let mut llvm_mod = self.ctx.create_module(self.spark[module].name.as_str());

        self.forward_funs(&mut llvm_mod, module);
        
        let defs = self.spark[module].defs.clone();

        self.current_scope.push_layer();

        for (name, def) in defs.iter() {
            self.current_scope.define(name.clone(), ScopeDef::Def(*def));
        }

        for (name, def) in self.spark[module].defs.iter() {
            if let SparkDef::FunDef(fun) = def {
                if let Some(body) = self.spark[*fun].body.as_ref() {
                    
                }
            }
        }

        self.current_scope.pop_layer();

        llvm_mod
    }
    
    /// Generate code for all function prototypes
    fn forward_funs(&mut self, llvm: &mut Module<'ctx>, module: ModId) {
        let defs = self.spark[module].defs.clone();

        for fun_id in defs
            .iter()
            .filter_map(|(_, def)| if let SparkDef::FunDef(id) = def { Some(*id) } else { None }) {
            let fun = self.spark[fun_id].clone();
            let llvm_fun_ty = self.gen_fun_ty(&fun.ty);
            let llvm_fun = llvm.add_function(fun.name.as_str(), llvm_fun_ty, Some(Linkage::External));
            self.llvm_funs.insert(fun_id, llvm_fun);
        }
    }

    /// Create an LLVM type from a type ID
    fn llvm_ty(&mut self, id: TypeId) -> AnyTypeEnum<'ctx> {
        match self.spark[id].data.clone() {
            TypeData::Integer { signed: _, width } => match width {
                IntegerWidth::Eight => self.ctx.i8_type().into(),
                IntegerWidth::Sixteen => self.ctx.i16_type().into(),
                IntegerWidth::ThirtyTwo => self.ctx.i32_type().into(),
                IntegerWidth::SixtyFour => self.ctx.i64_type().into(),
            },
            TypeData::Bool => self.ctx.bool_type().into(),
            TypeData::Tuple(elems) => {
                let elems = elems
                    .iter()
                    .map(|id| BasicTypeEnum::try_from(self.llvm_ty(*id)).unwrap())
                    .collect::<Vec<_>>();
                self.ctx.struct_type(&elems, false).into()
            },
            TypeData::Struct { fields } => {
                let fields = fields
                    .iter()
                    .map(|(id, _)| BasicTypeEnum::try_from(self.llvm_ty(*id)).unwrap())
                    .collect::<Vec<_>>();
                self.ctx.struct_type(&fields, false).into()
            },
            TypeData::Alias(id) => self.llvm_ty(id),
            TypeData::Pointer(id) => 
                BasicTypeEnum::try_from(self.llvm_ty(id))
                    .unwrap()
                    .ptr_type(AddressSpace::Generic)
                    .into(),
            TypeData::Array { element, len } => 
                BasicTypeEnum::try_from(self.llvm_ty(element))
                    .unwrap()
                    .array_type(len as u32)
                    .into(),
            TypeData::Unit => self.ctx.void_type().into(),
            TypeData::Invalid => unreachable!(),
            TypeData::Float { doublewide } => match doublewide {
                true => self.ctx.f64_type().into(),
                false => self.ctx.f32_type().into(),
            },
            TypeData::Function(ty) => self.gen_fun_ty(&ty).into(),
            TypeData::Enum { parts } => {
                let parts = parts
                    .iter()
                    .map(|ty| self.llvm_ty(*ty))
                    .collect::<Vec<_>>();
                let max_size = parts.iter().map(|ty| {
                        if let Ok(ty) = BasicTypeEnum::try_from(*ty) {
                            ty.size_of().map(|i| i.get_zero_extended_constant().unwrap() as u32).unwrap_or(0)
                        } else {
                            0
                        }
                    })
                    .max()
                    .unwrap();

                let field_types = &[self.ctx.i8_type().into(), self.ctx.i8_type().array_type(max_size).into()];

                self.ctx.struct_type(field_types, true).into() 
            }
        }

    }
    
    /// Create an LLVM function type from a spark IR function type
    fn gen_fun_ty(&mut self, ty: &FunctionType) -> InkwellFunctionType<'ctx> {
        let return_ty = self.llvm_ty(ty.return_ty);
        let args = ty
            .args
            .iter()
            .map(|ty| BasicTypeEnum::try_from(self.llvm_ty(*ty)).unwrap().into())
            .collect::<Vec<_>>();
        match return_ty {
            AnyTypeEnum::VoidType(return_ty) => 
                return_ty.fn_type(&args, false),
            _ => BasicTypeEnum::try_from(return_ty)
                .unwrap()
                .fn_type(&args, false)
        }

    }
}


