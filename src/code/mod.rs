//! The `code` module contains submodules handling linking and LLVM code generation
pub mod linker;
pub mod llvm;
pub mod ns;
pub use llvm::Compiler;

