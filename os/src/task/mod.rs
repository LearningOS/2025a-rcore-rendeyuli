//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the whole operating system.
//!
//! A single global instance of [`Processor`] called `PROCESSOR` monitors running
//! task(s) for each core.
//!
//! A single global instance of `PID_ALLOCATOR` allocates pid for user apps.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.
mod context;
mod id;
mod manager;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;


use crate::{loader::get_app_data_by_name, 
    mm::{MapPermission, VirtPageNum, VirtAddr},
};
use alloc::sync::Arc;
use lazy_static::*;
pub use manager::{fetch_task, TaskManager};
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
pub use id::{kstack_alloc, pid_alloc, KernelStack, PidHandle};
pub use manager::add_task;
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
    Processor,
};
/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        println!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );
        panic!("All applications completed!");
    }

    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    /// Creation of initial process
    ///
    /// the name "initproc" may be changed to any other app name like "usertests",
    /// but we have user_shell, so we don't need to change it.
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("ch5b_initproc").unwrap()
    ));
}

///Add init process to the manager
pub fn add_initproc() {
    add_task(INITPROC.clone());
}

/// 为当前任务映射一段虚拟地址空间
pub fn map_for_current_task(start_vpn:VirtPageNum,num_pages:usize, map_perm:MapPermission) -> isize {
    let current_task = current_task().unwrap();
    let memory_set = &mut current_task.inner_exclusive_access().memory_set;
    let mut end_vpn = start_vpn;
    for _ in 0..num_pages {
        if let Some(pte) = memory_set.translate(end_vpn){
            //这里我犯了一个错误，pte如果是valid的话，说明以及被映射过了，所以不应该再map,要映射的是invalid的pte
            if pte.is_valid() {
                return -1;
            }
        }
        end_vpn.0 += 1;
    }
    let start_va = VirtAddr::from(start_vpn);
    let end_va = VirtAddr::from(end_vpn);
    memory_set.insert_framed_area(start_va, end_va, map_perm);
    0
}

/// 取消当前任务的一段虚拟地址空间映射
pub fn unmap_for_current_task(start_vpn:VirtPageNum,num_pages:usize) -> isize {
    let current_task = current_task().unwrap();
    let memory_set = &mut current_task.inner_exclusive_access().memory_set;
    let mut end_vpn = start_vpn;
    for _ in 0..num_pages {
        if let Some(pte) = memory_set.translate(end_vpn){
            if !pte.is_valid() {
                return -1;
            }
        }
        memory_set.unmap_from_page_table(end_vpn);
        end_vpn.0 += 1;
    }
    //memory_set.remove_area_with_start_vpn(start_vpn);
    0
}