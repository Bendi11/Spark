use std::convert::{TryFrom, TryInto};

use hashbrown::HashMap;
use inkwell::{values::{BasicValueEnum, PointerValue, ArrayValue, CallableValue}, types::{BasicType, BasicTypeEnum}, FloatPredicate, IntPredicate};

use crate::{ir::{IrContext, value::{IrExpr, IrExprKind, IrLiteral}, types::{IrType, IrIntegerType}}, parse::token::Op};

use super::{LLVMCodeGeneratorState, LLVMCodeGenerator};



impl<'files, 'llvm> LLVMCodeGeneratorState<'files, 'llvm> {
    fn array_vals<T: TryFrom<BasicValueEnum<'llvm>>>(&mut self, irctx: &IrContext, vals: &[IrExpr]) -> Vec<T> 
    where 
        <T as TryFrom<BasicValueEnum<'llvm>>>::Error: std::fmt::Debug 
    {
        vals
            .iter()
            .map(|val| self.gen_expr(irctx, val).try_into().unwrap())
            .collect::<Vec<_>>()
    }

    ///Generate LLVM bytecode for a single IR expression
    pub fn gen_expr(&mut self, irctx: &IrContext, expr: &IrExpr) -> BasicValueEnum<'llvm> {
        match &expr.kind {
            IrExprKind::Var(v) => {
                let alloca = self.gen_lval(irctx, expr);
                self.build.build_load(alloca, "var_load")
            },
            IrExprKind::Lit(lit) => match lit {
                IrLiteral::Integer(v, ty) => self
                    .ctx
                    .i64_type()
                    .const_int(v.val, v.sign)
                    .const_cast(LLVMCodeGenerator::gen_inttype(&self.ctx, ty), ty.signed)
                    .into(),
                IrLiteral::Float(f, ty) => self
                    .ctx
                    .f64_type()
                    .const_float(*f)
                    .const_cast(if ty.doublewide { self.ctx.f64_type() } else { self.ctx.f32_type() })
                    .into(),
                IrLiteral::Bool(b) => self
                    .ctx
                    .bool_type()
                    .const_int(if *b { 1 } else { 0 }, false)
                    .into(),
                IrLiteral::Array(vals) => {
                    let ty = expr.ty;
                    let elem = if let IrType::Array(ty, _) = &irctx[ty] {
                        *ty
                    } else { unreachable!() };

                    

                    match self
                        .llvm_types
                        .get_secondary(elem) {
                            BasicTypeEnum::ArrayType(ty) => ty.const_array(&self.array_vals(irctx, vals)),
                            BasicTypeEnum::PointerType(ty) => ty.const_array(&self.array_vals(irctx, vals)),
                            BasicTypeEnum::StructType(ty) => ty.const_array(&self.array_vals(irctx, vals)),
                            BasicTypeEnum::FloatType(ty) => ty.const_array(&self.array_vals(irctx, vals)),
                            BasicTypeEnum::IntType(ty) => ty.const_array(&self.array_vals(irctx, vals)),
                            BasicTypeEnum::VectorType(ty) => ty.const_array(&self.array_vals(irctx, vals)),
                        }.into()
                        
                },
                IrLiteral::Unit => self.ctx.i8_type().const_int(0, false).into(),
                IrLiteral::Struct(s) => {
                    let irty = if let IrType::Struct(s_ty) = &irctx[expr.ty] {
                        s_ty
                    } else {
                        unreachable!()
                    };

                    let ty = self.llvm_types.get_secondary(expr.ty).into_struct_type();
                    let mut fields = HashMap::new();
                    for (name, field) in s.iter() {
                        let idx = irty.field_idx(name).unwrap();
                        fields.insert(idx, self.gen_expr(irctx, field));
                    }

                    let mut field_vec = vec![];
                    for i in 0..s.len() {
                        field_vec.push(*fields.get(&i).unwrap());
                    }

                    ty.const_named_struct(&field_vec).into()
                },
                IrLiteral::String(s) => {
                    self
                        .build
                        .build_global_string_ptr(s.as_str(), "strlit")
                        .as_pointer_value()
                        .into()
                        
                }
            }
            IrExprKind::Call(fun, args) => {
                let fun = self.gen_expr(irctx, fun).into_pointer_value();
                let callable = CallableValue::try_from(fun).unwrap();
                let args = args
                    .iter()
                    .map(|arg| self.gen_expr(irctx, arg).into())
                    .collect::<Vec<_>>();

                self
                    .build
                    .build_call(callable, &args, "call")
                    .try_as_basic_value()
                    .left()
                    .unwrap_or(self.ctx.i8_type().const_int(0, false).into())
            },
            IrExprKind::Fun(..) | IrExprKind::Member(..) | IrExprKind::Index(..) => self.gen_lval(irctx, expr).into(),
            IrExprKind::Cast(expr, ty) => {
                let llvm_expr = self.gen_expr(irctx, expr);
                if irctx.unwrap_alias(expr.ty) == irctx.unwrap_alias(*ty) {
                    llvm_expr
                } else {
                    todo!("{} is not the same as {}", irctx.typename(expr.ty), irctx.typename(*ty))
                }
            },
            IrExprKind::Unary(op, expr) => match op {
                Op::AND => self.gen_lval(irctx, expr).into(),
                _ => todo!(),
            }
            _ => todo!("{:?} is not yet implemented", expr)
        }
    }
    
    /// Generate code for an Lvalue
    pub fn gen_lval(&mut self, irctx: &IrContext, expr: &IrExpr) -> PointerValue<'llvm> {
        match &expr.kind {
            IrExprKind::Var(v) => self.llvm_vars.get_secondary(*v).unwrap(),
            IrExprKind::Fun(f) => self
                .llvm_funs
                .get_secondary(*f)
                .as_global_value()
                .as_pointer_value(),
            IrExprKind::Member(obj, field) => {
                let obj = self.gen_lval(irctx, obj);

                unsafe{ self
                    .build
                    .build_in_bounds_gep(obj, &[self.ctx.i64_type().const_zero(), self.ctx.i32_type().const_int(*field as u64, false)], "struct_gep")
                    //.unwrap()
                }
            },
            IrExprKind::Index(arr, elem) => {
                let arr = self.gen_lval(irctx, arr);
                let elem = self.gen_expr(irctx, elem);
                unsafe {
                    self
                        .build
                        .build_in_bounds_gep(
                            arr,
                            &[self.ctx.i32_type().const_zero(), elem],
                            "arr_index"
                        )
                        .into()
                }
            },
            _ => {
                let alloca = self.build.build_alloca(*self.llvm_types.get_secondary(expr.ty), "lval_alloca");
                let val = self.gen_expr(irctx, expr);
                self.build.build_store(alloca, val);
                alloca.into()
            }
        }
    }

    
    /// Generate LLVM bytecode for a binary expression
    pub fn gen_bin(&mut self, irctx: &IrContext, lhs: &IrExpr, op: Op, rhs: &IrExpr) -> BasicValueEnum<'llvm> {
        let llvm_lhs = self.gen_expr(irctx, lhs);
        let llvm_rhs = self.gen_expr(irctx, rhs);

        match (&irctx[lhs.ty], op, &irctx[rhs.ty]) {
            (IrType::Integer(IrIntegerType { signed, .. }), _, IrType::Integer(_)) if lhs.ty == rhs.ty => {
                let llvm_lhs = llvm_lhs.into_int_value();
                let llvm_rhs = llvm_rhs.into_int_value();
                match (op, *signed) {
                    (Op::Star, _) => self.build.build_int_mul(llvm_lhs, llvm_rhs, "imul").into(),
                    (Op::Div, true) => self.build.build_int_signed_div(llvm_lhs, llvm_rhs, "idiv").into(),
                    (Op::Div, false) => self.build.build_int_unsigned_div(llvm_lhs, llvm_rhs, "udiv").into(),
                    (Op::Add, _) => self.build.build_int_add(llvm_lhs, llvm_rhs, "iadd").into(),
                    (Op::Sub, _) => self.build.build_int_sub(llvm_lhs, llvm_rhs, "isub").into(),
                    (Op::ShRight, _) => self.build.build_right_shift(llvm_lhs, llvm_rhs, *signed, "ishift").into(),
                    (op @ (Op::Greater |
                        Op::GreaterEq | 
                        Op::Less | 
                        Op::LessEq | 
                        Op::Eq
                    ), 
                        _
                    ) => self.build.build_int_compare(
                            match (op, *signed) {
                                (Op::Greater, true) => IntPredicate::SGT,
                                (Op::GreaterEq, true) => IntPredicate::SGE,
                                (Op::Less, true) => IntPredicate::SLT,
                                (Op::LessEq, true) => IntPredicate::SLE,

                                (Op::Greater, false) => IntPredicate::UGT,
                                (Op::GreaterEq, false) => IntPredicate::UGE,
                                (Op::Less, false) => IntPredicate::ULT,
                                (Op::LessEq, false) => IntPredicate::ULE,

                                (Op::Eq, _) => IntPredicate::EQ,
                                _ => unreachable!()
                            },
                            llvm_lhs,
                            llvm_rhs,
                            "icmp",
                        ).into(),
                    _ => unreachable!(),
                }
            },
            (IrType::Float(_), op, IrType::Float(_)) => {
                let llvm_lhs = llvm_lhs.into_float_value();
                let llvm_rhs = llvm_rhs.into_float_value();
                match op {
                    Op::Star => self.build.build_float_mul(llvm_lhs, llvm_rhs, "fmul").into(),
                    Op::Div => self.build.build_float_div(llvm_lhs, llvm_rhs, "fdiv").into(),
                    Op::Add => self.build.build_float_add(llvm_lhs, llvm_rhs, "fadd").into(),
                    Op::Sub => self.build.build_float_sub(llvm_lhs, llvm_rhs, "fsub").into(),
                    Op::Mod => self.build.build_float_rem(llvm_lhs, llvm_rhs, "frem").into(),
                    op @ (Op::Greater |
                        Op::GreaterEq | 
                        Op::Less | 
                        Op::LessEq | 
                        Op::Eq) => self.build.build_float_compare(match op {
                                Op::Greater => FloatPredicate::OGT,
                                Op::GreaterEq => FloatPredicate::OGE,
                                Op::Less => FloatPredicate::OLT,
                                Op::LessEq => FloatPredicate::OLE,
                                Op::Eq => FloatPredicate::OEQ,
                                _ => unreachable!(),
                            },
                            llvm_lhs,
                            llvm_rhs,
                            "fcmp"
                        ).into(),
                    _ => unreachable!()
                }
            }
        }
    }
}
