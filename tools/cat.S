	
#if defined(__linux__)
#define MAIN main
#elif defined(__APPLE__)
#define MAIN _main
#else
#error "Unimplemented"
#endif

.global MAIN
.align 2

.text
MAIN:	

  // 00. Set buffer address in x0
  // Since BrainFuck has a memory space

#if defined(__linux__)
  adrp x0, buffer              // Load the buffer page address
  add  x0, x0, :lo12:buffer    // Add the page offset
#elif defined(__APPLE__)
  adrp x0, buffer@PAGE         // Load the buffer page address
  add  x0, x0, buffer@PAGEOFF  // Add the page offset
#else
#error "Unimplemented"
#endif

  // 0. write the INPUT (,) brainfuck block
  mov  x4, x0         // Save x0 value
  mov  x1, x0         // Set buffer address in x1
  mov  x0, #0         // Set x0 to 0 (read from stdin)
  mov  x2, #1         // Set x2 to 1 (read one byte only)
#if defined(__linux__)
  mov  x8, #63         // Set x16 to 63 (SYSCALL read)
#elif defined(__APPLE__)
  mov x16, #3         // Set x16 to 3 (SYSCALL read)
#else
#error "Unimplemented"
#endif
  svc  #0             // Trigger kernel trap
  cbnz x0, read_ok    // Check if we've read anything
  mov  w8, #0
  strb w8, [x4]       // Store 0 at [x0] (x4)
read_ok:
  mov  x0, x4         // Restore x0 value


  // 1. is the end of string reached?
  ldrb w8, [x0]
  cbz  w8, exit

loop:
  // 2. output character if any OUPUT (.) brainfuck block
  mov  x1, x0         // Set buffer
  mov  x4, x0         // Save x0
  mov  x0, #0x1       // Write to stdout
  mov  x2, #0x1       // Write one char
#if defined(__linux__)
  mov  x8, #64        // Syscall sys_write
#elif defined(__APPLE__)
  mov x16, #0x4       // Syscall sys_write
#else
#error "Unimplemented"
#endif
  svc  0x0            // Kernel trap
  mov  x0, x4         // Restore x0

  // 3. INPUT (,) brainfuck block
  mov  x4, x0         // Save x0 value
  mov  x1, x0         // Set buffer address in x1
  mov  x0, #0         // Set x0 to 0 (read from stdin)
  mov  x2, #1         // Set x2 to 1 (read one byte only)
#if defined(__linux__)
  mov  x8, #63         // Set x16 to 63 (SYSCALL read)
#elif defined(__APPLE__)
  mov x16, #3         // Set x16 to 3 (SYSCALL read)
#else
#error "Unimplemented"
#endif
  svc  #0             // Trigger kernel trap
  cbnz x0, read_ok_1  // Check if we've read anything
  mov  w8, #0
  strb w8, [x4]       // Store 0 at [x0] (x4)
read_ok_1:
  mov  x0, x4         // Restore x0 value

  // 4. loop if last read char was not \x00
  ldrb w8, [x0]
  cbnz w8, loop

exit:
  mov x0, #69              // Success exit status 0

exit_with_status:
#if defined(__linux__)
  mov x8, #93        // Set x16 to 93 (SYSCALL exit)
#elif defined(__APPLE__)
  mov x16, #1         // Set x16 to 1 (SYSCALL exit)
#else
#error "Unimplemented"
#endif
  svc #0

.data
.align 4
buffer:
    .space 16               // Allocate 16 bytes of zero-filled memory

