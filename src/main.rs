use anyhow::Result;
use clap::{command, Parser};
use spinoff::{spinners, Color, Spinner};
use std::{ffi::OsStr, path::PathBuf, str::FromStr, time::Duration};

const DEVICE_PATH: &str = "/dev/disk/by-label/";
const SLEEP_DURATION: Duration = Duration::from_secs(1);
const MAX_SCANS: &u8 = &50;

enum Target {
    #[allow(dead_code)]
    Device(PathBuf),
    Mountpoint(PathBuf),
}

struct Disk {
    target: Target,
    label: String,
}

fn list_mounted_disks(disks: &mut sysinfo::Disks) -> Vec<Disk> {
    disks.refresh_list();
    disks
        .iter()
        .map(|d| Disk {
            label: d.name().to_string_lossy().into_owned(),
            target: Target::Mountpoint(d.mount_point().to_path_buf()),
        })
        .collect()
}

fn list_unmounted_disks() -> Vec<Disk> {
    let labels = std::fs::read_dir(DEVICE_PATH).unwrap();
    labels
        .filter_map(|entry| entry.ok())
        .map(|l| {
            let label = l.file_name().to_string_lossy().into_owned();
            let device_path = PathBuf::from_str(&format!("{DEVICE_PATH}/{label}")).unwrap();
            Disk {
                label,
                target: Target::Device(device_path),
            }
        })
        .collect()
}

fn wait_for_disk(mount: bool, remaining: &[String]) -> Result<Disk> {
    let mut disks = sysinfo::Disks::new();

    for _ in 0..*MAX_SCANS {
        let disks = match mount {
            false => list_mounted_disks(&mut disks),
            true => list_unmounted_disks(),
        };

        for disk in disks {
            if remaining.contains(&disk.label) {
                return Ok(disk);
            }
        }

        std::thread::sleep(SLEEP_DURATION);
    }

    anyhow::bail!("Reached max retry count.")
}

#[cfg(target_os = "linux")]
fn mount_disk(path: PathBuf, label: &str) -> Result<sys_mount::Mount> {
    let mount_path = format!("/tmp/{}_mnt", label);

    if let Err(err) = std::fs::create_dir(&mount_path) {
        if err.kind() != std::io::ErrorKind::AlreadyExists {
            anyhow::bail!("Error while creating temporary mount directory: {:?}", err)
        }
    }

    let mount = sys_mount::Mount::builder()
        .fstype("vfat")
        .mount(path, mount_path)
        .unwrap();

    Ok(mount)
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'f', long)]
    file: PathBuf,

    #[arg(short = 'm', long)]
    mount: bool,

    #[arg(short = 'l', long, default_value = "GLV80LHBOOT")]
    left_hand_destination: String,

    #[arg(short = 'r', long, default_value = "GLV80RHBOOT")]
    right_hand_destination: String,
}

fn spinner(msg: &str) -> Spinner {
    Spinner::new(spinners::Dots, msg.to_string(), Color::Magenta)
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.file.exists() {
        anyhow::bail!("{:?} does not exist!", args.file)
    }

    if args.file.extension().and_then(OsStr::to_str) != Some("uf2") {
        anyhow::bail!("invalid file format")
    }

    let mount = if cfg!(target_os = "linux") {
        args.mount
    } else {
        false
    };

    #[cfg(target_os = "linux")]
    if args.mount {
        elevate::escalate_if_needed().expect("to escalate privileges");
    }

    let mut remaining = vec![args.left_hand_destination, args.right_hand_destination];
    let mut waiting = spinner("Waiting for devices...");

    while !remaining.is_empty() {
        let disk = wait_for_disk(mount, &remaining)?;
        waiting.clear();

        let mut status = spinner(&format!("{} - Connected", disk.label));

        let mount_point = match disk.target {
            Target::Mountpoint(path) => path,
            #[cfg(target_os = "linux")]
            Target::Device(path) => {
                status.update_text(format!("{} - Mounting device...", disk.label));
                let handle = match mount_disk(path, &disk.label) {
                    Ok(handle) => handle,
                    Err(err) => {
                        status.fail(&format!("{} - Unable to mount device", disk.label));
                        anyhow::bail!("Error: {:#}", err)
                    }
                };
                status.update_text(format!("{} - Mounted device", disk.label));
                handle.target_path().to_path_buf()
            }
            #[allow(unreachable_patterns)]
            _ => anyhow::bail!("invalid state"),
        };

        let filename = &args.file.file_name().expect("valid filename");
        std::fs::copy(&args.file, mount_point.join(filename)).unwrap();
        status.success(&format!("{} - Succesfully copied firmware", disk.label));

        remaining.retain(|d| d != &disk.label);

        if !remaining.is_empty() {
            waiting = spinner("Waiting for devices...");
        }
    }

    println!("Firmware update complete!");

    Ok(())
}
