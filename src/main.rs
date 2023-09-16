pub mod kvm;

use kvm::vm;
fn main() {
    vm::create_vm();
}