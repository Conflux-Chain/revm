#![allow(dead_code)]
use dynamic_host_macro::use_dyn_host;

trait Host {
    fn dummy_method(&self);
}

#[use_dyn_host]
fn test_function<H: Host + ?Sized>(interpreter: &mut u32, host: &mut H) {
    host.dummy_method();
    *interpreter += 1;
}

#[use_dyn_host]
fn test_function_with_generic<const N: usize, H: Host + ?Sized>(interpreter: &mut u32, host: &mut H) {
    host.dummy_method();
    *interpreter += 1;
}


fn main() {}