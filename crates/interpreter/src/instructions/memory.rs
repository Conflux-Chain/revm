use crate::{
    gas,
    primitives::{Spec, U256},
    Host, Interpreter,
};
use core::cmp::max;

use dynamic_host_macro::use_dyn_host;


#[use_dyn_host]
pub fn mload<H: Host + ?Sized>(interpreter: &mut Interpreter, _host: &mut H) {
    gas!(interpreter, gas::VERYLOW);
    pop_top!(interpreter, top);
    let offset = as_usize_or_fail!(interpreter, top);
    resize_memory!(interpreter, offset, 32);
    *top = interpreter.shared_memory.get_u256(offset);
}

#[use_dyn_host]
pub fn mstore<H: Host + ?Sized>(interpreter: &mut Interpreter, _host: &mut H) {
    gas!(interpreter, gas::VERYLOW);
    pop!(interpreter, offset, value);
    let offset = as_usize_or_fail!(interpreter, offset);
    resize_memory!(interpreter, offset, 32);
    interpreter.shared_memory.set_u256(offset, value);
}

#[use_dyn_host]
pub fn mstore8<H: Host + ?Sized>(interpreter: &mut Interpreter, _host: &mut H) {
    gas!(interpreter, gas::VERYLOW);
    pop!(interpreter, offset, value);
    let offset = as_usize_or_fail!(interpreter, offset);
    resize_memory!(interpreter, offset, 1);
    interpreter.shared_memory.set_byte(offset, value.byte(0))
}

#[use_dyn_host]
pub fn msize<H: Host + ?Sized>(interpreter: &mut Interpreter, _host: &mut H) {
    gas!(interpreter, gas::BASE);
    push!(interpreter, U256::from(interpreter.shared_memory.len()));
}

// EIP-5656: MCOPY - Memory copying instruction
#[use_dyn_host]
pub fn mcopy<H: Host + ?Sized, SPEC: Spec>(interpreter: &mut Interpreter, _host: &mut H) {
    check!(interpreter, CANCUN);
    pop!(interpreter, dst, src, len);

    // into usize or fail
    let len = as_usize_or_fail!(interpreter, len);
    // deduce gas
    gas_or_fail!(interpreter, gas::verylowcopy_cost(len as u64));
    if len == 0 {
        return;
    }

    let dst = as_usize_or_fail!(interpreter, dst);
    let src = as_usize_or_fail!(interpreter, src);
    // resize memory
    resize_memory!(interpreter, max(dst, src), len);
    // copy memory in place
    interpreter.shared_memory.copy(dst, src, len);
}
