# C2Rust Guidelines

author: 孙桢波 黄冰鉴

date: 2019/12/31

---



### Preface

2019年秋季学期，我们选修了操作系统专题训练课。我们选择的课程项目是`VideoCore IV GPU Driver for rCore on Raspberry Pi 3B+`，在[rCore](<https://github.com/rcore-os/rCore>)上实现树莓派的GPU驱动。原有的可参考的GPU驱动实现是用C编写的，因此我们做的事情是理解驱动工作原理，并将驱动从C移植到Rust。在移植的过程中，我们遇到了诸多困难，有时是因为C和Rust不同的语言特性，有时是因为C和Rust对同一语言特性的定义不同。我们最终完成了移植的工作。为了方便之后其它人开展类似的移植工作，我们尝试总结出一些通用的移植方法和注意事项，以供参考。



---



### Section 1: Biggest Difference--Ownership, Lifetime, and Borrow

如果要说什么是C和Rust最大的区别的话，我们认为是所有权，生命周期以及借用。

Rust的安全性保证来源于它在编译时就能发现大部分的安全问题。它能做到这点就归功于上面的三个特性。

所有权保证了在同一时间点，一个对象只能存在一个可变引用(mut)。因此C中常见的将多个指针指向同一变量的地址的操作在Rust中是unsafe的。为保证多个reference访问的安全性，我们使用了Arc和Mutex，典型例子如下：

```rust
//返回值是Arc<Mutex<gpu_bo>>，Arc和Mutex可以避免冲突访问。
//使用clone返回reference pointer。
pub fn vc4_lookup_bo(&self, handle: u32) -> Option<Arc<Mutex<gpu_bo>>> {
    ...
    Some(bo.clone())
}
```



C->C++->Rust的OOP优化

在编写C内核时，有大量的函数参数是指针类型，此时使用面向对象OOP的方法能够有效避免无谓的传递参数，增强安全性。我们认为这种实现方法是具有通用性的。

```c
//file: vc4_bo.c 
//函数vc4_bo_create调用了to_vc4_dev，作用是获得指向device的成员变量driver_data的指针。
//这样的操作在rust里是不安全的，因为driver_data将同时拥有两个指针。
struct vc4_bo *vc4_bo_create(struct device *dev, size_t size,
			     enum vc4_kernel_bo_type type)
{
	struct vc4_dev *vc4 = to_vc4_dev(dev);
	...
}
static inline struct vc4_dev *to_vc4_dev(struct device *dev)
{
	return (struct vc4_dev *)dev->driver_data;
}
```

```rust
//我们的实现方法
//将vc4_bo_create作为GpuDevice的函数方法，用&mut self替代原来的device *dev，避免了传参和所有权传递。所有的操作直接指向对象本身。
impl GpuDevice {
	...
	pub fn vc4_bo_create(&mut self, size: u32, bo_type: u32)->Option<Arc<Mutex<gpu_bo>>>
	{...}
	...
}
```



---



### Section 2: Details

传递参数和返回结果的建议

依然是因为所有权和生命周期等原因，在传递参数和返回结果时要格外注意变量的类型。

我们使用了如下方法：

- 灵活使用None和Some(Object)传递参数

  - 在C的系统编程中存在大量的函数，如果结果正常则返回值是指向对象的指针，如果结果异常则返回NULL。
  - 在Rust中并没有NULL这一类型，真正对应的实现是None/Option/Some()。

- 灵活使用Ok()和Err()传递返回值

  - 在C的系统编程中存在大量的函数，结果正常时返回一个值，结果错误时返回错误码。

  - 在Rust中可以采用Ok()和Err()的机制更好地表示返回值。

  - 以mailbox::mem_alloc()为例，它的返回值是Ok(ret[0])，既包含了Ok，又嵌入了正常返回值的信息，增加了信息量。

    ```rust
    pub fn mem_alloc(size: u32, align: u32, flags: u32) -> PropertyMailboxResult<u32> {
        let ret = send_one_tag!(RPI_FIRMWARE_ALLOCATE_MEMORY, [size, align, flags])?;
        Ok(ret[0])
    }
    ```

    当某一处调用mem_alloc时，可以通过Ok过滤掉错误结果，同时取出Ok内的结果handle。

    ```rust
    //somewhere call mem_alloc()
    if let Ok(handle) = mailbox::mem_alloc(size, PAGE_SIZE as u32, mailbox::MEM_FLAG_COHERENT | mailbox::MEM_FLAG_ZERO) {...}
    ```




宏定义的建议

众所周知，C程序中存在大量无类型的宏定义，在Rust中并没有对应的实现。因此，我们的解决方式是就事论事，根据宏的实际使用场景定义了函数的类型。以Roundup为例，在C中它的定义如下：

```c
/* Round up to the nearest multiple of n */
#define ROUNDUP(a, n) ({                                            \
    size_t __n = (size_t)(n);                               \
    (typeof(a))(ROUNDDOWN((size_t)(a) + __n - 1, __n));     \
})
```

而在Rust中，我们根据Roundup的实际使用场景，确定类型为u32，进行了定义：

```rust
pub fn roundUp(a:u32, n:u32) -> u32 {
	roundDown(a + n - 1, n)
}
```





---



### Section 3: Unavoidable situations

写Rust时目标是尽量避免unsafe语句的使用。但是既然是内核编程，不可避免会存在使用unsafe的地方。比如在我们编写的驱动中，涉及对Rendering Control List的填充，此时就需要用到unsafe的操作，比如：

```rust
pub fn cl_u8(&mut self, data: u8) {
	unsafe {
		*(self.next as *mut u8) = data;
	}
	self.next += 1;
}
```

虽然我们在此处使用了unsafe的操作，但是因为函数功能非常清楚，所以问题不大。

从全局来看，我们只使用了19处unsafe操作，因此我们认为我们很好地保证了程序的安全性。

我们也可以仿照bcm2837里的实现，构造对象来处理一段连续内存空间。在初始化时将这段空间映射到对应的成员变量上，之后对这段内存的读写都是safe的。


---



### Section 4: Some Tips

在定义结构体时，因为C和Rust的结构体存储方式不同，为了避免出现相关问题，要加上声明`#[repr(C)]`。

在定义函数时，如果希望Rust程序编译成的链接库还能重新被C程序使用，函数定义需要加上如下内容：

```rust
#[no_mangle]
pub extern "C" fn pass_str
```