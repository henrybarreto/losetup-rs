use libc::{O_RDWR, close, ioctl, open};
use std::{
    ffi::CString,
    fs::File,
    io::{Error, Result},
    os::fd::{AsFd, RawFd},
};

const LO_NAME_SIZE: usize = 64;
const LO_KEY_SIZE: usize = 32;

/// Represents the status information of a loop device (64-bit version).
///
/// This structure corresponds to the `loop_info64` struct used in the Linux kernel.
/// It holds metadata about a loop device, including file association, encryption
/// details, and offset/size limits.
///
/// Fields like `lo_file_name` and `lo_crypt_name` are null-terminated byte arrays
/// and may require conversion to strings for readable output.
///
/// See `LOOP_GET_STATUS64` and `LOOP_SET_STATUS64` for ioctl usage.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LoopInfo64 {
    lo_device: u64,
    lo_inode: u64,
    lo_rdevice: u64,
    lo_offset: u64,
    lo_sizelimit: u64,
    lo_number: u32,
    lo_encrypt_type: u32,
    lo_encrypt_key_size: u32,
    lo_flags: u32,
    lo_file_name: [u8; LO_NAME_SIZE],
    lo_crypt_name: [u8; LO_NAME_SIZE],
    lo_encrypt_key: [u8; LO_KEY_SIZE],
    lo_init: [u64; 2],
}

impl Default for LoopInfo64 {
    fn default() -> Self {
        Self {
            lo_device: 0,
            lo_inode: 0,
            lo_rdevice: 0,
            lo_offset: 0,
            lo_sizelimit: 0,
            lo_number: 0,
            lo_encrypt_type: 0,
            lo_encrypt_key_size: 0,
            lo_flags: 0,
            lo_file_name: [0; LO_NAME_SIZE],
            lo_crypt_name: [0; LO_NAME_SIZE],
            lo_encrypt_key: [0; LO_KEY_SIZE],
            lo_init: [0; 2],
        }
    }
}

/// Sets up a loop device by associating it with a file descriptor
pub const LOOP_SET_FD: u64 = 0x4C00;
/// Clears a loop device, disassociating it from its backing file
pub const LOOP_CLR_FD: u64 = 0x4C01;
/// Sets status information for a loop device (legacy version)
pub const LOOP_SET_STATUS: u64 = 0x4C02;
/// Gets status information from a loop device (legacy version)
pub const LOOP_GET_STATUS: u64 = 0x4C03;
/// Sets status information for a loop device with 64-bit structure
pub const LOOP_SET_STATUS64: u64 = 0x4C04;
/// Gets status information from a loop device with 64-bit structure
pub const LOOP_GET_STATUS64: u64 = 0x4C05;
/// Changes the backing file descriptor for a loop device
pub const LOOP_CHANGE_FD: u64 = 0x4C06;
/// Sets the capacity (size) of the loop device
pub const LOOP_SET_CAPACITY: u64 = 0x4C07;
/// Enables or disables direct I/O on the loop device
pub const LOOP_SET_DIRECT_IO: u64 = 0x4C08;
/// Sets the block size for the loop device
pub const LOOP_SET_BLOCK_SIZE: u64 = 0x4C09;
/// Configures multiple loop device parameters in a single operation
pub const LOOP_CONFIGURE: u64 = 0x4C0A;

// /dev/loop-control interface
/// Adds a new loop device to the system
pub const LOOP_CTL_ADD: u64 = 0x4C80;
/// Removes a loop device from the system
pub const LOOP_CTL_REMOVE: u64 = 0x4C81;
/// Gets the number of the next available free loop device
pub const LOOP_CTL_GET_FREE: u64 = 0x4C82;

/// A Simple losetup implementation for managing Linux loop devices.
///
/// Loop devices allow regular files to be accessed as block devices, which is
/// useful for mounting disk images and creating virtual filesystems.
pub struct Losetup {
    fd: RawFd,
}

impl Losetup {
    /// Creates a new `Losetup` instance by opening the loop control device.
    ///
    /// This function opens the `/dev/loop-control` device, which is used to
    /// manage loop devices in Linux.
    ///
    /// # Returns
    ///
    /// A `Result` containing a new `Losetup` instance on success, or an error
    /// if the loop control device could not be opened.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The `/dev/loop-control` device does not exist
    /// - The user does not have sufficient permissions
    /// - The system does not support loop devices
    pub fn open() -> Result<Self> {
        let fd = unsafe { open(CString::new("/dev/loop-control")?.as_ptr(), O_RDWR) };
        if fd < 0 {
            return Err(Error::last_os_error());
        }

        Ok(Self { fd })
    }

    /// Finds the next available loop device.
    ///
    /// Uses the `LOOP_CTL_GET_FREE` ioctl to request the next free loop
    /// device number from the kernel.
    ///
    /// # Returns
    ///
    /// A `Result` containing the path to the next available loop device
    /// (e.g., `/dev/loop0`) on success, or an error if no free device
    /// could be found.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - All loop devices are in use
    /// - The `ioctl` call fails for any reason
    ///
    /// # Examples
    ///
    /// ```
    /// use losetup_rs::Losetup;
    ///
    /// let loopctl = Losetup::open().unwrap();
    /// let device = loopctl.next_free().unwrap();
    ///
    /// println!("Next free loop device: {}", device);
    /// ```
    pub fn next_free(&self) -> Result<String> {
        let mut loop_num: i32 = -1;

        let res = unsafe { ioctl(self.fd, LOOP_CTL_GET_FREE, &mut loop_num) };
        if res < 0 {
            return Err(Error::last_os_error());
        }

        Ok(format!("/dev/loop{}", loop_num))
    }

    /// Attaches a file to a loop device.
    ///
    /// This function associates a file with a loop device, making the
    /// contents of the file accessible as a block device.
    ///
    /// # Parameters
    ///
    /// * `device` - The path to the loop device (e.g., `/dev/loop0`)
    /// * `path` - The path to the file to be attached
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The loop device could not be opened
    /// - The specified file could not be opened
    /// - The `ioctl` call to attach the file fails
    /// - The user does not have sufficient permissions
    ///
    /// # Examples
    ///
    /// ```
    /// use losetup_rs::Losetup;
    ///
    /// let loopctl = Losetup::open().unwrap();
    /// let device = loopctl.next_free().unwrap();
    ///
    /// loopctl.attach(&device, "/path/to/disk.img").unwrap();;
    /// ```
    ///
    /// # Note
    ///
    /// The file will remain attached until explicitly detached with
    /// [`Losetup::detach`] or until the system is rebooted.
    pub fn attach(&self, device: &str, path: &str) -> Result<()> {
        let loop_fd = unsafe { open(CString::new(device)?.as_ptr(), O_RDWR) };
        if loop_fd < 0 {
            return Err(Error::last_os_error());
        }

        let file = File::open(path)?;
        let file_fd = file.as_fd();
        let res = unsafe { ioctl(loop_fd, LOOP_SET_FD, file_fd) };
        if res < 0 {
            unsafe { close(loop_fd) };
            return Err(Error::last_os_error());
        }

        unsafe { close(loop_fd) };

        Ok(())
    }

    /// Detaches a file from a loop device.
    ///
    /// This function disassociates a previously attached file from a loop device,
    /// making the loop device available for reuse.
    ///
    /// # Parameters
    ///
    /// * `device` - The path to the loop device to detach (e.g., `/dev/loop0`)
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an error if the operation failed.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The loop device could not be opened
    /// - The `ioctl` call to detach the file fails
    /// - The device is still in use (e.g., mounted)
    /// - The user does not have sufficient permissions
    ///
    /// # Examples
    ///
    /// ```
    /// use losetup_rs::Losetup;
    ///
    /// let loopctl = Losetup::open().unwrap();
    /// let device = loopctl.next_free().unwrap();
    ///
    /// loopctl.attach(&device, "/path/to/disk.img").unwrap();;
    ///
    /// // operate over attached device.
    ///
    /// loopctl.detach(&device).unwrap();
    /// ```
    ///
    /// # Note
    ///
    /// It's important to detach loop devices when they are no longer needed
    /// to free up system resources. Ensure that any filesystems mounted on
    /// the loop device are unmounted before detaching.
    pub fn detach(&self, device: &str) -> Result<()> {
        let loop_fd = unsafe { open(CString::new(device)?.as_ptr(), O_RDWR) };
        if loop_fd < 0 {
            return Err(Error::last_os_error());
        }

        let res = unsafe { ioctl(loop_fd, LOOP_CLR_FD) };
        unsafe { close(loop_fd) };
        if res < 0 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }

    /// Retrieves the status of a loop device.
    ///
    /// This function uses the `LOOP_GET_STATUS64` ioctl command to query the
    /// configuration and metadata associated with a loop device.
    ///
    /// # Parameters
    ///
    /// * `device` - The path to the loop device (e.g., `/dev/loop0`)
    ///
    /// # Returns
    ///
    /// A `Result` containing a `LoopInfo64` structure on success, or an error
    /// if the operation failed.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The loop device could not be opened
    /// - The ioctl call fails (e.g., if the device is not in use)
    /// - The user does not have sufficient permissions
    ///
    /// # Examples
    ///
    /// ```
    /// use losetup_rs::Losetup;
    ///
    /// let device = "/dev/loop0";
    /// match Losetup::status(device) {
    ///     Ok(info) => println!("Device is active: {:?}", info),
    ///     Err(err) => eprintln!("Failed to get status: {}", err),
    /// }
    /// ```
    pub fn status(device: &str) -> Result<LoopInfo64> {
        let fd = unsafe { open(CString::new(device)?.as_ptr(), O_RDWR) };
        if fd < 0 {
            return Err(Error::last_os_error());
        }

        let mut info = LoopInfo64::default();

        let res = unsafe { ioctl(fd, LOOP_GET_STATUS64, &mut info) };
        if res < 0 {
            return Err(Error::last_os_error());
        }

        Ok(info)
    }
}

impl Drop for Losetup {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}
