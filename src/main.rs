pub mod kvm;

use kvm::vm;
fn main() {
    let vm = vm::Vm::new().unwrap();
    vm.run();
}
