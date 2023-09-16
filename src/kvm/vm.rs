use kvm_bindings::{kvm_userspace_memory_region, KVM_MEM_LOG_DIRTY_PAGES};
use kvm_ioctls::{Kvm, VmFd};
use libc;
use std::ptr::null_mut;

pub struct Vm {
    fd: VmFd,
    mem_size: usize,
    guest_addr: u64,
}

impl Vm {
    pub fn new() -> Result<Self, &'static str> {
        println!("Create vm...");

        // Open `/dev/kvm` internally
        let kvm = Kvm::new().unwrap();

        let kvm_api_v = kvm.get_api_version();
        println!("kvm api version: {}", kvm_api_v);

        // Create VM
        // Call ioctl with KVM_CREATE_VM internally
        let vm_fd = kvm.create_vm().unwrap();

        // prepare guest memory
        // Call ioctl with KVM_SET_USER_MEMORY_REGION internally
        let mem_size = 0x4000;
        let guest_addr = 0x1000;
        let load_addr: *mut u8 = unsafe {
            libc::mmap(
                null_mut(),
                mem_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_SHARED | libc::MAP_NORESERVE,
                -1,
                0,
            ) as *mut u8
        };
        let mem_region = kvm_userspace_memory_region {
            slot: 0,
            guest_phys_addr: guest_addr,
            memory_size: mem_size as u64,
            userspace_addr: load_addr as u64,
            flags: KVM_MEM_LOG_DIRTY_PAGES,
        };
        unsafe {
            vm_fd.set_user_memory_region(mem_region).unwrap();
        }

        Ok(Vm {
            fd: vm_fd,
            mem_size: mem_size,
            guest_addr: guest_addr,
        })
    }
}
