use losetup_rs::Losetup;
use std::{fs::File, io::Write, path::Path};

fn create_test_file(path: &str, size: usize) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.set_len(size as u64)?;
    writeln!(file, "Test content")?;

    Ok(())
}

fn main() -> std::io::Result<()> {
    let img_path = "/tmp/test_loop.img";

    if !Path::new(img_path).exists() {
        println!("Creating test file at {}", img_path);

        create_test_file(img_path, 10 * 1024 * 1024)?; // 10 MB
    }

    let loopctl = Losetup::open()?;

    let device = loopctl.next_free()?;
    dbg!(&device);
    println!("Using loop device: {}", device);

    loopctl.attach(&device, img_path)?;
    println!("Attached {} to {}", img_path, device);

    // Do something useful here (e.g., mount, read, etc.)

    loopctl.detach(&device)?;
    println!("Detached {}", device);

    Ok(())
}
