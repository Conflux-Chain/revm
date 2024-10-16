#![allow(clippy::wrong_self_convention)]

use super::instruction;
use crate::{instructions::control, primitives::Spec, Host, Interpreter};
use std::boxed::Box;

/// EVM opcode function signature.
pub type Instruction = for<'h> fn(&mut Interpreter, &mut (dyn 'h + Host));

/// Instruction table is list of instruction function pointers mapped to 256 EVM opcodes.
pub type InstructionTable = [Instruction; 256];

/// EVM dynamic opcode function signature.
pub type DynInstruction<'a> = dyn for<'h> Fn(&mut Interpreter, &mut (dyn 'h + Host)) + 'a;

/// EVM boxed dynamic opcode function signature.
pub type BoxedInstruction<'a> = Box<DynInstruction<'a>>;

/// A table of boxed instructions.
pub type BoxedInstructionTable<'a> = [BoxedInstruction<'a>; 256];

/// Either a plain, static instruction table, or a boxed, dynamic instruction table.
///
/// Note that `Plain` variant is about 10-20% faster in Interpreter execution.
pub enum InstructionTables<'a> {
    Plain(InstructionTable),
    Boxed(BoxedInstructionTable<'a>),
}

impl<'a> InstructionTables<'a> {
    /// Creates a plain instruction table for the given spec. See [`make_instruction_table`].
    #[inline]
    pub const fn new_plain<SPEC: Spec>() -> Self {
        Self::Plain(make_instruction_table::<SPEC>())
    }
}

impl<'a> InstructionTables<'a> {
    /// Inserts the instruction into the table with the specified index.
    #[inline]
    pub fn insert(&mut self, opcode: u8, instruction: Instruction) {
        match self {
            Self::Plain(table) => table[opcode as usize] = instruction,
            Self::Boxed(table) => table[opcode as usize] = Box::new(instruction),
        }
    }

    /// Converts the current instruction table to a boxed variant if it is not already, and returns
    /// a mutable reference to the boxed table.
    #[inline]
    pub fn to_boxed(&mut self) -> &mut BoxedInstructionTable<'a> {
        self.to_boxed_with(|i| Box::new(i))
    }

    /// Converts the current instruction table to a boxed variant if it is not already with `f`,
    /// and returns a mutable reference to the boxed table.
    #[inline]
    pub fn to_boxed_with<F>(&mut self, f: F) -> &mut BoxedInstructionTable<'a>
    where
        F: FnMut(Instruction) -> BoxedInstruction<'a>,
    {
        match self {
            Self::Plain(_) => self.to_boxed_with_slow(f),
            Self::Boxed(boxed) => boxed,
        }
    }

    #[cold]
    fn to_boxed_with_slow<F>(&mut self, f: F) -> &mut BoxedInstructionTable<'a>
    where
        F: FnMut(Instruction) -> BoxedInstruction<'a>,
    {
        let Self::Plain(table) = self else {
            unreachable!()
        };
        *self = Self::Boxed(make_boxed_instruction_table(table, f));
        let Self::Boxed(boxed) = self else {
            unreachable!()
        };
        boxed
    }

    /// Returns a mutable reference to the boxed instruction at the specified index.
    #[inline]
    pub fn get_boxed(&mut self, opcode: u8) -> &mut BoxedInstruction<'a> {
        &mut self.to_boxed()[opcode as usize]
    }

    /// Inserts a boxed instruction into the table at the specified index.
    #[inline]
    pub fn insert_boxed(&mut self, opcode: u8, instruction: BoxedInstruction<'a>) {
        *self.get_boxed(opcode) = instruction;
    }

    /// Replaces a boxed instruction into the table at the specified index, returning the previous
    /// instruction.
    #[inline]
    pub fn replace_boxed(
        &mut self,
        opcode: u8,
        instruction: BoxedInstruction<'a>,
    ) -> BoxedInstruction<'a> {
        core::mem::replace(self.get_boxed(opcode), instruction)
    }

    /// Updates a single instruction in the table at the specified index with `f`.
    #[inline]
    pub fn update_boxed<F>(&mut self, opcode: u8, f: F)
    where
        F: for<'h> Fn(&DynInstruction<'a>, &mut Interpreter, &mut (dyn 'h + Host)) + 'a,
    {
        update_boxed_instruction(self.get_boxed(opcode), f)
    }

    /// Updates every instruction in the table by calling `f`.
    #[inline]
    pub fn update_all<F>(&mut self, f: F)
    where
    F: for<'h> Fn(&DynInstruction<'a>, &mut Interpreter, &mut (dyn 'h + Host)) + Copy +'a,
    {
        // Don't go through `to_boxed` to avoid allocating the plain table twice.
        match self {
            Self::Plain(_) => {
                self.to_boxed_with(|prev| Box::new(move |i, h| f(&prev, i, h)));
            }
            Self::Boxed(boxed) => boxed
                .iter_mut()
                .for_each(|instruction| update_boxed_instruction(instruction, f)),
        }
    }
}

/// Make instruction table.
#[inline]
pub const fn make_instruction_table<SPEC: Spec>() -> InstructionTable {
    // Force const-eval of the table creation, making this function trivial.
    // TODO: Replace this with a `const {}` block once it is stable.
    struct ConstTable<SPEC: Spec> {
        _spec: core::marker::PhantomData<SPEC>,
    }
    impl<SPEC: Spec> ConstTable<SPEC> {
        const NEW: InstructionTable = {
            let mut tables: InstructionTable = [control::unknown; 256];
            let mut i = 0;
            while i < 256 {
                tables[i] = instruction::<SPEC>(i as u8);
                i += 1;
            }
            tables
        };
    }
    ConstTable::<SPEC>::NEW
}

/// Make boxed instruction table that calls `f` closure for every instruction.
#[inline]
pub fn make_boxed_instruction_table<'a, FN>(
    table: &InstructionTable,
    mut f: FN,
) -> BoxedInstructionTable<'a>
where
    FN: FnMut(Instruction) -> BoxedInstruction<'a>,
{
    core::array::from_fn(|i| f(table[i]))
}

/// Updates a boxed instruction with a new one.
#[inline]
pub fn update_boxed_instruction<'a, F>(instruction: &mut BoxedInstruction<'a>, f: F)
where
    F: for<'h> Fn(&DynInstruction<'a>, &mut Interpreter, &mut (dyn 'h + Host)) + 'a,
{
    // NOTE: This first allocation gets elided by the compiler.
    let prev = core::mem::replace(instruction, Box::new(|_, _| {}));
    *instruction = Box::new(move |i, h| f(&prev, i, h));
}
