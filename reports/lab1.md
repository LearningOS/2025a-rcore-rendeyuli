代码实现了syscall的sys_trace功能，能根据syscall_id以及传入的args执行不同的功能

关于任务的切换，在__switch里，通过传人两个参数，current_task_cx_ptr,next_task_cx_ptr,分别存入a0/a1，表示的是task_cx_ptr,即任务上下文的指针

其实本章多道程序这里主要就两点，__switch,__restore
__switch:内核态任务之间的切换,对应的是内核栈的保存和恢复
__restore: 内核态<-->用户态的切换，准确来说是trap.S里的实现，
即__alltraps:保存用户态的状态，用于后续恢复用户态的状态，然后转入内核，用户态->内核态
__restore: 恢复用户态状态，切换回用户态，内核态->用户态
另外，__restore也可以恢复内核态任务信息
对应的是用户栈的保存和恢复

在syscall/mod.rs下：
    // 每次进入内核时，增加当前任务id的系统调用计数
    inc_syscall_count(syscall_id);
去增加任务的syscall_id的值
在syscall/process.rs下实现sys_trace三种模式的匹配处理
在task/mod.rs下实现任务的系统调用次数的增加和查询
在task/task.rs下实现TaskConControlBlock里的syscall_counts统计任务不同系统调用id对应的调用次数

## 简答作业
1.
文件名	含义	程序行为
ch2b_bad_address.rs	Bad Address（页错误 PageFault）测试程序	故意访问非法地址（如 *(0x0 as *mut u8) = 0;）来触发 PageFault
ch2b_bad_instructions.rs	Illegal Instruction（非法指令）测试程序	执行 CPU 不支持或未定义的指令（如用 asm!("unimp")）
ch2b_bad_register.rs	Bad Register（寄存器错误）测试程序	故意让寄存器非法使用，比如调用系统调用时参数错误、或跳转到无效地址
2.
(1)
刚执行到__restore时，sp指针内容如下：
pc             0x80201424       0x80201424 <__restore>
sp             0x80206ef0       0x80206ef0 <os::loader::KERNEL_STACK+7920>
继续si，观察sp的值
__restore使用场景:Trap内核态<->用户态，Task内核态之间的切换

(2)
特殊处理了CSR寄存器：sstatus,sepc,sscratch
sstatus: cpu将当前的特权级按照sstatus的spp段设置的恢复为U/S态
sepc: 存Trap处理完成后默认会执行的下一条指令的地址
sscratch: 保存“另一方”的栈指针，在__restore时，sscratch里读取sp指向的用户态的栈指针，再交换，此时sp->用户态，sscratch->内核态

(3)
x2是sp,不是在内存读入的，而是后面的csrrw sp,scratch,sp
x4是tp线程指针，没有使用多线程

(4)
sp存的用户栈指针
sscratch存的内核栈指针

(5)
sret
由risc-v文档知，sret会从sstatus.SPP里获取“上次的特权级”，SPP=0->U,SPP=1->S,同时sstatus的SIE字段设置为SPIE，SPIE设置为1，spp是指为0->U,这样CPU就回到了用户态

SIE 位用于在管理模式下启用或禁用所有中断。SIE=1,允许在S模式下中断，SIE=0，禁止在S模式下中断
SPIE 位用于指示在进入S模式之前是否已启用S中断。使能中断位，保存SIE值的
sret时，从SPIE->SIE恢复SIE的值，同时SPIE=1，为下一次Trap做准备

The SPP bit indicates the privilege level at which a hart was executing before entering supervisor mode. When a trap is taken, SPP is set to 0 if the trap originated from user mode, or 1 otherwise. When an SRET instruction (see [otherpriv]) is executed to return from the trap handler, the privilege level is set to user mode if the SPP bit is 0, or supervisor mode if the SPP bit is 1; SPP is then set to 0.

(6)
交换了sp和sscratch的值，sp此时->内核栈指针，sscratch存储的用户栈指针，sp就指向了用户栈

(7)
call trap_handler
因为执行了系统调用，切换到S态，而syscall函数中实现了ecall指令
硬件层次的执行


在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

《你交流的对象说明》 无

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

《你参考的资料说明》 RISC-V手册

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。