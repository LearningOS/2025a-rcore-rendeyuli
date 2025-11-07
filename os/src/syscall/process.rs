//! Process management syscalls
//use riscv::addr::page;

use crate::{
    mm::{PageTable, VirtAddr}, task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_syscall_count, suspend_current_and_run_next, get_current_task_id,
    }
};
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    -1
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
// 这个系统调用有三种功能，根据 trace_request 的值不同，执行不同的操作：
// 如果 trace_request 为 0，则 id 应被视作 *const u8 ，表示读取当前任务 id 地址处一个字节的无符号整数值。此时应忽略 data 参数。返回值为 id 地址处的值。
// 如果 trace_request 为 1，则 id 应被视作 *mut u8 ，表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0。
// 如果 trace_request 为 2，表示查询当前任务调用编号为 id 的系统调用的次数，返回值为这个调用次数。本次调用也计入统计 。
// 否则，忽略其他参数，返回值为 -1。
// 此外，由于本章我们有了地址空间作为隔离机制，所以 sys_trace 需要考虑一些额外的情况：
// 在读取（trace_request 为 0）时，如果对应地址用户不可见或不可读，则返回值应为 -1（isize 格式的 -1，而非 u8）。
// 在写入（trace_request 为 1）时，如果对应地址用户不可见或不可写，则返回值应为 -1（isize 格式的 -1，而非 u8）。

pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let token = current_user_token();
    let page_table = PageTable::from_token(token);
    match trace_request{
        0 => {
            let va = VirtAddr(id as *const u8 as usize);
            let vpn = va.floor();
            match page_table.translate(vpn){
                Some(pte) => {
                    if !pte.is_valid() || !pte.readable() {
                        -1
                    } else {
                        // 读取地址处一个字节的值并返回
                        let ppn = pte.ppn();
                        ppn.get_bytes_array()[va.page_offset()] as isize
                    }
                },
                None => {
                    -1
                }
            }
        },
        1 => {
            let va = VirtAddr(id as *const u8 as usize);
            let vpn = va.floor();
            match page_table.translate(vpn){
                Some(pte) => {
                    if !pte.is_valid() || !pte.writable() {
                        -1
                    } else {
                        let ppn = pte.ppn();
                        ppn.get_bytes_array()[va.page_offset()] = data as u8;
                        0
                    }
                }
                None => {
                    -1
                }
            }

        },
        2 => {
            let syscall_id = id;
            let task_id = get_current_task_id();
            get_syscall_count(task_id, syscall_id) as isize
        },
        _ => {
            -1
        }
    }
    //-1
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
