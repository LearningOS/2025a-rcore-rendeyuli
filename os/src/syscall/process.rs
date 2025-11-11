//! Process management syscalls
//use riscv::addr::page;

use crate::{
    config::PAGE_SIZE, mm::{PageTable, VirtAddr,MapPermission, translated_byte_buffer}, task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_syscall_count, map_for_current_task, suspend_current_and_run_next,unmap_for_current_task
    }, timer::get_time_us
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
    let us = get_time_us();
    let ts = TimeVal {
        sec: us/1000000,
        usec: us%1000000,
    };
    let ptr = _ts as *const u8;
    let len = core::mem::size_of::<TimeVal>();
    //这里转换为字符串切片考虑到了跨页的问题
    //buffer里存储的结构可能是多个切片比如：
    //{&mut [u8]//2个字节, &mut [u8]//6个字节} 总共8个字节
    let buffers = translated_byte_buffer(current_user_token(), ptr, len);
    // 将ts写入到用户空间的内存
    // 先转换为字符切片
    let _ts_ptr_arr = unsafe {
        core::slice::from_raw_parts(
            &ts as *const TimeVal as *const u8,
            len   
        )
    };
    let mut ts_index: usize = 0;
    for buffer in buffers{
        buffer.copy_from_slice(&_ts_ptr_arr[ts_index..ts_index+buffer.len()]);
        ts_index += buffer.len();
    }

    0
    //-1
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
            let va = VirtAddr(id as *const u8 as usize);//一个指向字节的地址，取出数后又转换成无符号整数
            let vpn = va.floor();
            match page_table.translate(vpn){
                Some(pte) => {
                    if !pte.is_valid() || !pte.readable() || !pte.useraccessible() {
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
                    if !pte.is_valid() || !pte.writable() || !pte.useraccessible(){
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
            get_syscall_count(id) as isize
        },
        _ => {
            -1
        }
    }
    //-1
}

// 申请长度为 len 字节的物理内存（不要求实际物理内存位置，可以随便找一块），将其映射到 start 开始的虚存，内存页属性为 prot
// 参数：
// start 需要映射的虚存起始地址，要求按页对齐
// len 映射字节长度，可以为 0
// prot：第 0 位表示是否可读，第 1 位表示是否可写，第 2 位表示是否可执行。其他位无效且必须为 0
// 返回值：执行成功则返回 0，错误返回 -1
// 说明：
// 为了简单，目标虚存区间要求按页对齐，len 可直接按页向上取整，不考虑分配失败时的页回收。


//可能的错误：
// start 没有按页大小对齐
// prot & !0x7 != 0 (prot 其余位必须为0)
// prot & 0x7 = 0 (这样的内存无意义)
// [start, start + len) 中存在已经被映射的页
// 物理内存不足

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if start % 4096 != 0 {
        return -1;
    }
    if port & 0x7 == 0 || port & !0x7 != 0 {
        return -1;
    }
    let start_vpn = VirtAddr::from(start).floor();
    let num_pages = (len-1+PAGE_SIZE) / PAGE_SIZE;
    let mut map_perm: MapPermission = MapPermission::U;
    if port & 0x1 != 0 {
        map_perm |= MapPermission::R;
    }
    if port & 0x2 != 0{
        map_perm |= MapPermission::W;
    }
    if port & 0x4 != 0{
        map_perm |= MapPermission::X;
    }
    match map_for_current_task(start_vpn, num_pages,map_perm) {
        0 => {
            return 0;
        }
        _ => {
            return -1;
        }
    }

    //-1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if start % 4096 != 0 {
        return -1;
    }
    let start_vpn = VirtAddr::from(start).floor();
    let num_pages = (len-1+PAGE_SIZE) / PAGE_SIZE;
    match unmap_for_current_task(start_vpn,num_pages){
        0 => {
            return 0;
        }
        _ => {
            return -1;
        }
    }
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
