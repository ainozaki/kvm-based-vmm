use core::slice;
use kvm_bindings::{kvm_userspace_memory_region, KVM_MEM_LOG_DIRTY_PAGES};
use kvm_ioctls::{Kvm, VcpuExit, VcpuFd, VmFd};
use libc;
use std::{io::Write, ptr::null_mut};

pub struct Vm {
    vm_fd: VmFd,
    vcpu_fd: VcpuFd,
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

        // Write code in the guest memory
        let asm_code: &[u8];
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            asm_code = &[
                0xba, 0xf8, 0x03, /* mov $0x3f8, %dx */
                0x00, 0xd8, /* add %bl, %al */
                0x04, b'0', /* add $'0', %al */
                0xee, /* out %al, %dx */
                0xec, /* in %dx, %al */
                0xc6, 0x06, 0x00, 0x80,
                0x00, /* movl $0, (0x8000); This generates a MMIO Write. */
                0x8a, 0x16, 0x00, 0x80, /* movl (0x8000), %dl; This generates a MMIO Read. */
                0xf4, /* hlt */
            ];
        }
        unsafe {
            let mut slice = slice::from_raw_parts_mut(load_addr, mem_size);
            slice.write(&asm_code).unwrap();
        }

        // Create vCPU
        // Call ioctl with KVM_CREATE_VCPU internally
        let vcpu_fd = vm_fd.create_vcpu(0).unwrap();

        // Initialize registers
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // specific registers
            let mut vcpu_sregs = vcpu_fd.get_sregs().unwrap();
            vcpu_sregs.cs.base = 0;
            vcpu_sregs.cs.selector = 0;
            vcpu_fd.set_sregs(&vcpu_sregs).unwrap();

            // general registers
            let mut vcpu_regs = vcpu_fd.get_regs().unwrap();
            vcpu_regs.rip = guest_addr;
            vcpu_regs.rax = 2;
            vcpu_regs.rbx = 3;
            vcpu_regs.rflags = 2;
            vcpu_fd.set_regs(&vcpu_regs).unwrap();
        }

        Ok(Vm {
            vm_fd: vm_fd,
            vcpu_fd: vcpu_fd,
            mem_size: mem_size,
            guest_addr: guest_addr,
        })
    }

    pub fn run(&self) {
        println!("run!");
        loop {
            match self.vcpu_fd.run().expect("run failed") {
                VcpuExit::IoIn(addr, data) => {
                    println!(
                        "Received an I/O in exit. Address: {:#x}. Data: {:#x}",
                        addr, data[0],
                    )
                }
                VcpuExit::IoOut(addr, data) => {
                    println!(
                        "Received an I/O out exit. Address: {:#x}. Data: {:#x}",
                        addr, data[0],
                    );
                }
                VcpuExit::MmioRead(addr, data) => {
                    println!("Received an MMIO Read Request for the address {:#x}.", addr,);
                }
                VcpuExit::MmioWrite(addr, data) => {
                    println!("Received an MMIO Write Request to the address {:#x}.", addr,);
                    // The code snippet dirties 1 page when it is loaded in memory
                    let dirty_pages_bitmap = self.vm_fd.get_dirty_log(0, self.mem_size).unwrap();
                    let dirty_pages = dirty_pages_bitmap
                        .into_iter()
                        .map(|page| page.count_ones())
                        .fold(0, |dirty_page_count, i| dirty_page_count + i);
                    assert_eq!(dirty_pages, 1);
                }
                VcpuExit::Hlt => {
                    break;
                }
                r => panic!("Unexpected exit reason: {:?}", r),
            }
        }
    }
}
