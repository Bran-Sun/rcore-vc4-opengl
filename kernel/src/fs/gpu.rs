use rcore_fs::vfs::*;

use crate::drivers::gpu::gpu_device::GPU_DEVICE;
use crate::memory::phys_to_virt;
use alloc::{string::String, sync::Arc, vec::Vec};
use core::any::Any;
use super::vga::*;

#[derive(Default)]

pub struct Gpu;

impl INode for Gpu {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        Err(FsError::EntryNotFound)
    }
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        info!("the _offset is {} {}", _offset, _buf[0]);
        // let lock = FRAME_BUFFER.lock();
        // if let Some(ref frame_buffer) = *lock {
        //     use core::slice;
        //     let frame_buffer_data = unsafe {
        //         slice::from_raw_parts_mut(
        //             frame_buffer.base_addr() as *mut u8,
        //             frame_buffer.framebuffer_size(),
        //         )
        //     };
        //     frame_buffer_data.copy_from_slice(&_buf);
        //     Ok(frame_buffer.framebuffer_size())
        // } else {
        //     Err(FsError::EntryNotFound)
        // }
        Err(FsError::EntryNotFound)
    }
    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            // TOKNOW and TODO
            read: true,
            write: false,
            error: false,
        })
    }
    fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata {
            dev: 0,
            inode: 0,
            size: 0x24000,
            blk_size: 0,
            blocks: 0,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
            type_: FileType::SymLink,
            mode: 0,
            nlinks: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
        })
    }

    fn io_control(&self, cmd: u32, data: usize) -> Result<()> {
        #[cfg(target_arch = "aarch64")]
        {
            if let Some(gd) = GPU_DEVICE.lock().as_mut() {
                return gd.io_control(cmd, data)
            }
        }
        warn!("use never support ioctl !");
        Err(FsError::NotSupported)
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}
