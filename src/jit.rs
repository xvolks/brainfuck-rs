use std::{
    ffi::{CStr, c_char},
    fs,
    os::raw::c_void,
    ptr::null_mut,
};

use crate::{Cpu, Op};
use errno::errno;
use libc::{
    MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_EXEC, PROT_READ, PROT_WRITE, free,
    malloc, memcpy, memset, mmap, mprotect, munmap, strerror,
};

#[cfg(target_os = "macos")]
use libc::MAP_JIT;
#[cfg(not(target_os = "macos"))]
const MAP_JIT: i32 = 0; // Unused on non macos systems

const JIT_MEMORY_SIZE: usize = 1 * 1024 * 1024;

struct Backpatch {
    operand_byte_addr: usize,
    src_byte_addr: usize,
    dst_op_index: usize,
    origin: Op,
}

trait Buffer {
    fn push_bytes(&mut self, bytes: &[u8]);
    fn push_u32(&mut self, bytes: u32);
    fn push_u8(&mut self, byte: u8);
}

impl Buffer for Vec<u8> {
    fn push_bytes(&mut self, bytes: &[u8]) {
        for c in bytes.iter() {
            self.push(*c);
        }
    }
    fn push_u32(&mut self, bytes: u32) {
        let mut b = bytes;
        for _ in 0..4 {
            self.push((b & 0xff) as u8);
            b >>= 8;
        }
    }
    fn push_u8(&mut self, byte: u8) {
        self.push(byte);
    }
}

impl Op {
    #[cfg(target_arch = "x86_64")]
    fn inc(buffer: &mut Vec<u8>, operand: u32) {
        buffer.push_bytes(b"\x80\x07"); // add byte[rdi],
        buffer.push_u8(operand as u8);
    }
    #[cfg(target_arch = "aarch64")]
    fn inc(buffer: &mut Vec<u8>, operand: u32) {
        buffer.push_bytes(b"\x08\x00\x40\x39"); // ldrb w8, [x0]
        let add_op = 0x11000108 | (operand as u8) << 10;
        buffer.push_u32(add_op); // add w8, w8, #constant (operand)
        buffer.push_bytes(b"\x08\x00\x00\x39"); // strb w8, [x0]
    }
    #[cfg(target_arch = "x86_64")]
    fn dec(buffer: &mut Vec<u8>, operand: u32) {
        buffer.push_bytes(b"\x80\x2f"); // sub byte[rdi],
        buffer.push_u8(operand as u8);
    }
    #[cfg(target_arch = "aarch64")]
    fn dec(buffer: &mut Vec<u8>, operand: u32) {
        buffer.push_bytes(b"\x08\x00\x40\x39"); // ldrb w8, [x0]
        let sub_op = 0x51000108 | (operand as u8) << 10;
        buffer.push_u32(sub_op); // sub w8, w8, operand
        buffer.push_bytes(b"\x08\x00\x00\x39"); // strb w8, [x0]
    }

    #[cfg(target_arch = "x86_64")]
    fn left(buffer: &mut Vec<u8>, operand: u32) {
        buffer.push_bytes(b"\x48\x81\xef"); // sub rdi, operand
        buffer.push_u32(operand);
    }

    #[cfg(target_arch = "aarch64")]
    fn left(buffer: &mut Vec<u8>, operand: u32) {
        if operand >= 256 {
            todo!("TODO: support bigger operands");
        }
        let add_op = 0xd1000000 | (operand as u8) << 10;
        buffer.push_u32(add_op); // sub x0, x0, operand
    }

    #[cfg(target_arch = "x86_64")]
    fn right(buffer: &mut Vec<u8>, operand: u32) {
        buffer.push_bytes(b"\x48\x81\xc7"); // add rdi,
        buffer.push_u32(operand);
    }

    #[cfg(target_arch = "aarch64")]
    fn right(buffer: &mut Vec<u8>, operand: u32) {
        if operand >= 256 {
            todo!("TODO: support bigger operands");
        }
        let add_op = 0x91000000 | (operand as u8) << 10;
        buffer.push_u32(add_op); // add x0, x0, operand
    }

    #[cfg(target_arch = "x86_64")]
    fn out(buffer: &mut Vec<u8>) {
        buffer.push_bytes(b"\x57"); // push rdi
        if cfg!(target_os = "macos") {
            buffer.push_bytes(b"\x48\xc7\xc0\x04\x00\x00\x02"); // mov rax, 2000004
        }
        buffer.push_bytes(b"\x48\xc7\xc2\x01\x00\x00\x00"); // mov rdx, 1
        buffer.push_bytes(b"\x0f\x05"); // syscall
        buffer.push_bytes(b"\x5f"); // pop rdi
    }

    #[cfg(target_arch = "aarch64")]
    fn out(buffer: &mut Vec<u8>) {
        buffer.push_bytes(b"\xe1\x03\x00\xaa"); // mov x1, x0
        buffer.push_bytes(b"\xe4\x03\x00\xaa"); // mov x4, x0
        buffer.push_bytes(b"\x20\x00\x80\xd2"); // mov x0, 1
        buffer.push_bytes(b"\x22\x00\x80\xd2"); // mov x2, 1
        buffer.push_bytes(b"\x90\x00\x80\xd2"); // mov x16, 4
        buffer.push_bytes(b"\x01\x00\x00\xd4"); // svc 0
        buffer.push_bytes(b"\xe0\x03\x04\xaa"); // mov x0, x4
    }

    #[cfg(target_arch = "x86_64")]
    fn inp(buffer: &mut Vec<u8>) {
        buffer.push_bytes(b"\x57"); // push rdi
        if cfg!(target_os = "macos") {
            buffer.push_bytes(b"\x48\xc7\xc0\x03\x00\x00\x02"); // mov rax, 2000003
        }
        buffer.push_bytes(b"\x48\xc7\xc2\x01\x00\x00\x00"); // mov rdx, 1
        buffer.push_bytes(b"\x0f\x05"); // syscall
        buffer.push_bytes(b"\x5f"); // pop rdi
    }

    #[cfg(target_arch = "aarch64")]
    fn inp(buffer: &mut Vec<u8>) {
        buffer.push_bytes(b"\xe4\x03\x00\xaa"); //     mov        x4,x0
        buffer.push_bytes(b"\xe1\x03\x00\xaa"); //     mov        x1,x0
        buffer.push_bytes(b"\x00\x00\x80\xd2"); //     mov        x0,#0x0
        buffer.push_bytes(b"\x22\x00\x80\xd2"); //     mov        x2,#0x1
        buffer.push_bytes(b"\x70\x00\x80\xd2"); //     mov        x16,#0x3
        buffer.push_bytes(b"\x01\x00\x00\xd4"); //     svc        0x0
        buffer.push_bytes(b"\x60\x00\x00\xb5"); //     cbnz       x0,read_ok
        buffer.push_bytes(b"\x08\x00\x80\x52"); //     mov        w8,#0x0
        buffer.push_bytes(b"\x88\x00\x00\x39"); //     strb       w8,[x4]=>buffer
        // read_ok:
        buffer.push_bytes(b"\xe0\x03\x04\xaa"); //     mov        x0,x4
    }

    #[cfg(target_arch = "x86_64")]
    fn jz(buffer: &mut Vec<u8>, operand: usize, backpatches: &mut Vec<Backpatch>) {
        buffer.push_bytes(b"\x8a\x07"); // mov al, byte [rdi]
        buffer.push_bytes(b"\x84\xc0"); // test al, al
        buffer.push_bytes(b"\x0f\x84"); // jz
        let operand_byte_addr = buffer.len();
        buffer.push_bytes(b"\x00\x00\x00\x00");
        let src_byte_addr = buffer.len();
        let bp = Backpatch {
            operand_byte_addr,
            src_byte_addr,
            dst_op_index: operand,
            origin: Op::Jz(operand),
        };
        backpatches.push(bp);
    }

    #[cfg(target_arch = "aarch64")]
    fn jz(buffer: &mut Vec<u8>, operand: usize, backpatches: &mut Vec<Backpatch>) {
        buffer.push_bytes(b"\x08\x00\x40\x39"); // ldrb w8, [x0]
        let operand_byte_addr = buffer.len();
        let src_byte_addr = buffer.len();
        buffer.push_bytes(b"\x08\x00\x00\x34"); // cbz w8, <loc>
        let bp = Backpatch {
            operand_byte_addr,
            src_byte_addr,
            dst_op_index: operand,
            origin: Op::Jz(operand),
        };
        backpatches.push(bp);
    }

    #[cfg(target_arch = "x86_64")]
    fn jnz(buffer: &mut Vec<u8>, operand: usize, backpatches: &mut Vec<Backpatch>) {
        buffer.push_bytes(b"\x8a\x07"); // mov al, byte [rdi]
        buffer.push_bytes(b"\x84\xc0"); // test al, al
        buffer.push_bytes(b"\x0f\x85"); // jnz
        let operand_byte_addr = buffer.len();
        buffer.push_bytes(b"\x00\x00\x00\x00");
        let src_byte_addr = buffer.len();

        let bp = Backpatch {
            operand_byte_addr,
            src_byte_addr,
            dst_op_index: operand,
            origin: Op::Jnz(operand),
        };
        backpatches.push(bp);
    }

    #[cfg(target_arch = "aarch64")]
    fn jnz(buffer: &mut Vec<u8>, operand: usize, backpatches: &mut Vec<Backpatch>) {
        buffer.push_bytes(b"\x08\x00\x40\x39"); // ldrb w8, [x0]
        let operand_byte_addr = buffer.len();
        let src_byte_addr = buffer.len();
        buffer.push_bytes(b"\x08\x00\x00\x35"); // cbnz w8, <loc>
        let bp = Backpatch {
            operand_byte_addr,
            src_byte_addr,
            dst_op_index: operand,
            origin: Op::Jnz(operand),
        };
        backpatches.push(bp);
    }

    #[cfg(target_arch = "x86_64")]
    fn ret(buffer: &mut Vec<u8>) {
        buffer.push_bytes(b"\xc3");
    }

    #[cfg(target_arch = "aarch64")]
    fn ret(buffer: &mut Vec<u8>) {
        buffer.push_bytes(b"\xc0\x03\x5f\xd6");
    }
}

fn last_error() -> String {
    to_string(unsafe { strerror(errno().0) })
}

fn to_string(char_ptr: *mut c_char) -> String {
    unsafe { CStr::from_ptr(char_ptr) }
        .to_str()
        .unwrap()
        .to_string()
}

impl Cpu {
    pub fn execute_jit(&mut self) -> std::io::Result<()> {
        let ops: &[Op] = &self.ops;
        let mut native_mem = vec![];
        let mut instruction_addrs = vec![];
        let mut backpatches = vec![];
        for op in ops.iter() {
            // Store the address offset of each instruction block
            instruction_addrs.push(native_mem.len());
            match op {
                Op::Inc(n) => Op::inc(&mut native_mem, *n),
                Op::Dec(n) => Op::dec(&mut native_mem, *n),
                Op::Left(n) => Op::left(&mut native_mem, *n),
                Op::Right(n) => Op::right(&mut native_mem, *n),
                Op::In(n) => {
                    for _ in 0..*n {
                        Op::inp(&mut native_mem)
                    }
                }
                Op::Out(n) => {
                    for _ in 0..*n {
                        Op::out(&mut native_mem)
                    }
                }
                Op::Jz(n) => Op::jz(&mut native_mem, *n + 1, &mut backpatches),
                Op::Jnz(n) => Op::jnz(&mut native_mem, *n + 1, &mut backpatches),
            }
        }
        instruction_addrs.push(native_mem.len());
        Op::ret(&mut native_mem);

        for bp in backpatches.iter() {
            let src_addr = bp.src_byte_addr as i64;
            let dst_addr = instruction_addrs[bp.dst_op_index] as i64;
            let operand = dst_addr - src_addr;
            // #ifdef __aarch64__
            if cfg!(target_arch = "aarch64") {
                assert!(
                    (operand >= -0x40000 && operand <= 0x40000),
                    "TODO: branch too far"
                );
                // *(uint32_t*) &sb.items[bp.operand_byte_addr] |= (uint32_t) ((operand / 4) & 0x7ffff) << 5;
                // println!("origin: {:?}, operand: {:04x}", bp.origin, operand);
                let patch = ((operand / 4) & 0x7ffff) << 5;
                // println!("patch: {patch:04x}");
                for i in 0..4 {
                    // TODO: check this 🫠
                    native_mem[bp.operand_byte_addr + i] |= (patch >> (8 * i)) as u8 & 0xff;
                }
            } else if cfg!(target_arch = "x86_64") {
                for i in 0..4 {
                    native_mem[bp.operand_byte_addr + i] |= (operand >> (24 - 8 * i)) as u8 & 0xff;
                }
            } else {
                panic!("Unmanaged architecture");
            }
        }

        #[cfg(debug_assertions)]
        fs::write("ops.bin", &native_mem)?;

        #[cfg(debug_assertions)]
        println!("Mem: {:02x?}", native_mem);

        let mut map_flags = MAP_PRIVATE | MAP_ANONYMOUS;
        let mut map_prot = PROT_WRITE | PROT_READ;
        if cfg!(target_os = "macos") {
            map_flags |= MAP_JIT;
        } else {
            map_prot |= PROT_EXEC;
        }

        unsafe {
            let addr = mmap(null_mut(), native_mem.len(), map_prot, map_flags, -1, 0);
            if addr == MAP_FAILED {
                panic!("Could not allocate executable memory: {}", last_error());
            }
            memcpy(addr, native_mem.as_ptr() as *const c_void, native_mem.len());
            mprotect(addr, native_mem.len(), PROT_EXEC | PROT_READ);
            let pointer = addr; // as fn() as *mut c_void ();
            let brainfuck_code = std::mem::transmute::<*mut c_void, fn(*mut c_void)>(pointer);

            let memory = malloc(JIT_MEMORY_SIZE);
            if memory == null_mut() {
                eprintln!("Failed to malloc jit memory");
                munmap(addr, native_mem.len());
                return Err(std::io::Error::last_os_error());
            }
            memset(memory, 0, JIT_MEMORY_SIZE);
            brainfuck_code(memory);
            munmap(addr, native_mem.len());
            free(memory);
        }
        Ok(())
    }
}
