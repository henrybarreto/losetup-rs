use libc::{O_RDWR, close, ioctl, open};
use std::{
    ffi::CString,
    fs::File,
    io::{Error, Result},
    os::fd::{AsFd, RawFd},
};

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
///
/// # Examples
///
/// ```
/// use std::error::Error;
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     // Create a new loop control interface
///     let loopctl = Losetup::open()?;
///     
///     // Find the next available loop device
///     let device = loopctl.next_free()?;
///     println!("Available loop device: {}", device);
///     
///     // Attach a disk image to the loop device
///     Losetup::attach(&device, "/path/to/disk.img")?;
///     
///     // Later, detach the loop device
///     Losetup::detach(&device)?;
///     
///     Ok(())
/// }
/// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// let loopctl = Losetup::open()?;
    ///
    /// ```
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
    /// let loopctl = Losetup::open()?;
    /// let device = loopctl.next_free()?;
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
    /// let loopctl = Losetup::open()?;
    /// let device = loopctl.next_free()?;
    /// Losetup::attach(&device, "/path/to/disk.img")?;
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
    /// // After you're done using the loop device
    /// Losetup::detach("/dev/loop0")?;
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
}

impl Drop for Losetup {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}
