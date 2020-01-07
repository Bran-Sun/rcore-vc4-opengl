## C2Rust Tranlation Analysis



### Preface

我们小组在完成操作系统专题训练大作业的过程中，在陈渝老师的提醒下，发现了这个有趣的翻译工具。鉴于我们在翻译过程中积累了许多经验和教训，我们也希望对这个工具进行分析，看看其是否能作为翻译的参考，为以后做移植工作的同学提供方便。

黄冰鉴&孙桢波，2019/12/31



---



### Introduction

[C2Rust](<https://immunant.com/blog/2019/08/introduction-to-c2rust/>)是一个将C项目翻译成Rust项目的工具。

C2Rust翻译流程

```
Command 1: intercept-build make
跟踪c项目的make过程，生成compile_commands.json，内有对编译指令的参数描述。
​```
{
        "arguments": [
            "cc", 
            "-c", 
            "-std=c99", 
            "-o", 
            "test", 
            "test.c"
        ], 
        "directory": "/home/huangbj16/Documents/ostrain/buffer", 
        "file": "test.c"
    }, 
​```
command 2: c2rust transpile compile_commands.json
根据json完成翻译，生成rust项目
```



---



### 可以借鉴的地方

- 翻译的完整性，生成的是可直接运行的rust代码。
- 函数翻译满足FFI要求，
  - `#[no_mangle]
    pub extern "C" fn rust_function() {}`

- 很好地处理的goto的逻辑关系，重新组织了函数的内容逻辑。

- include".h"引用头文件的处理

  ```
  头文件中的函数声明，变为extern C
  original
      #include "vc4_cl.h"
      vc4_cl.h:
          void vc4_init_cl(struct vc4_cl *cl);
  C2Rust Translation
  	vc4_render_cl.rs
      extern "C" {
          #[no_mangle]
          fn vc4_bo_create(dev: *mut device, size: size_t,
                           type_0: vc4_kernel_bo_type) -> *mut vc4_bo;
      }
  ```



---



### 存在的问题

- 大量unsafe使用，几乎所有的函数都是unsafe的。

```
大量的raw pointers使用和dereference使用。
(*bo).size = size;
(*bo).handle = handle;
(*bo).paddr = bus_addr;
(*bo).vaddr = bus_addr as *mut libc::c_void;
(*bo).type_0 = type_0;
```

- 不支持子项目编译。如果用户想要翻译的是一个大项目中的子部分，因为C2Rust依赖于makefile，如果没有子项目的makefile，则无法实现翻译。在我们使用C2Rust的过程中，我们希望使用它翻译uCore中的一个文件夹vc4，因为它只支持局部翻译，我们花费了大量精力将vc4从uCore中剥离了出来，才实现了翻译，成本非常高。
- 头文件中的函数实现，翻译非常冗余，在每一个引用的地方都赋值粘贴了一遍。

```
头文件中的函数实现和Struct声明，直接复制粘贴，代码冗余。
original
	#include "vc4_cl.h"
    static inline void cl_u32(struct vc4_cl *cl, uint32_t n);
C2Rust translation
	vc4_render_cl.rs
	#[inline]
    unsafe extern "C" fn cl_u32(mut cl: *mut vc4_cl, mut n: uint32_t) {
        put_unaligned_32((*cl).next, n);
        cl_advance(cl, 4i32 as uint32_t);
    }
```

- 宏定义：因为C的宏定义没有指定类型，C2Rust根据每一处具体使用，进行了翻译，过于冗余。正确的做法是根据使用场景，对类型进行明确。

```rust
//未实现宏定义，直接代码覆盖：
//猜测原因：C的宏定义没有限定类型，在rust里无法定义。
//宏定义ROUNDUP in C
#define ROUNDUP(a, n) ({                                            \
            size_t __n = (size_t)(n);                               \
            (typeof(a))(ROUNDDOWN((size_t)(a) + __n - 1, __n));     \
        })
size = ROUNDUP(size, PGSIZE);
//宏定义ROUNDUP的使用 in C2Rust
let mut __a: size_t =
                      size.wrapping_add(__n).wrapping_sub(1i32 as
                                                              libc::c_ulonglong);
                  __a.wrapping_sub(__a.wrapping_rem(__n))
//我们的实现 in Rust，指定u32类型
pub fn roundUp(a:u32, n:u32) -> u32 {
	roundDown(a + n - 1, n)
}
```

- Enumeration

```rust
//对Enum的处理
//original in C
enum vc4_kernel_bo_type {
	VC4_BO_TYPE_FB,
	VC4_BO_TYPE_V3D,
	VC4_BO_TYPE_BIN,
	VC4_BO_TYPE_RCL,
	VC4_BO_TYPE_BCL,
};
//C2Rust
pub type vc4_kernel_bo_type = libc::c_uint;
pub const VC4_BO_TYPE_BCL: vc4_kernel_bo_type = 4;
pub const VC4_BO_TYPE_RCL: vc4_kernel_bo_type = 3;
pub const VC4_BO_TYPE_BIN: vc4_kernel_bo_type = 2;
pub const VC4_BO_TYPE_V3D: vc4_kernel_bo_type = 1;
pub const VC4_BO_TYPE_FB: vc4_kernel_bo_type = 0;
//our translation
enum vc4_kernel_bo_type {
	VC4_BO_TYPE_FB,
	VC4_BO_TYPE_V3D,
	VC4_BO_TYPE_BIN,
	VC4_BO_TYPE_RCL,
	VC4_BO_TYPE_BCL,
}
```

- 大量使用了libc的类型，而非rust原生类型。

```rust
//类型转换
pub type bool_0 = libc::c_int;
pub type uint32_t = libc::c_uint;
pub type uint64_t = libc::c_ulonglong;
pub type uintptr_t = uint64_t;
pub type size_t = uintptr_t;
pub type __u32 = uint32_t;
```



---



### Conclusion

总体而言，C2Rust是一个非常初步的翻译工具。它虽然能够直接将C项目翻译成可执行的Rust项目，但是翻译之后的项目只能作为参考，如果编程者想要进一步修改代码，难度非常大。因此，我们建议编程者仅仅将其作为参考，而非权威性的翻译工具。