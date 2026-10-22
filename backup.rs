// Build: rustc --edition=2021 backup.rs -o backup
use std::{
    env, fs,
    io::{self, Write},
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const HOME_PATHS: &[&str] = &[
    ".bashrc",
    ".bash_profile",
    ".bash_logout",
    ".profile",
    ".zshenv",
    ".zshrc",
    ".inputrc",
    ".XCompose",
    ".gtkrc-2.0",
    ".icons",
    ".themes",
    ".fonts",
    ".config/hypr",
    ".config/omarchy",
    ".config/alacritty",
    ".config/foot",
    ".config/kitty",
    ".config/ghostty",
    ".config/btop",
    ".config/fastfetch",
    ".config/lazygit",
    ".config/git",
    ".config/starship.toml",
    ".config/nvim",
    ".config/tmux",
    ".config/eza",
    ".config/imv",
    ".config/mpv",
    ".config/mise",
    ".config/fontconfig",
    ".config/gtk-3.0",
    ".config/gtk-4.0",
    ".config/qt5ct",
    ".config/qt6ct",
    ".config/Kvantum",
    ".config/xdg-desktop-portal",
    ".config/environment.d",
    ".config/autostart",
    ".config/systemd/user",
    ".config/fcitx5",
    ".config/voxtype",
    ".config/wireplumber",
    ".config/hyprland-preview-share-picker",
    ".config/xournalpp",
    ".config/mimeapps.list",
    ".config/user-dirs.dirs",
    ".config/user-dirs.locale",
    ".config/xdg-terminals.list",
    ".config/chromium-flags.conf",
    ".local/bin",
    ".local/libexec",
    ".local/share/fonts",
    ".local/share/icons",
    ".local/share/cursors",
    ".local/share/themes",
    ".local/share/applications",
    ".local/share/wallpapers",
    ".local/share/backgrounds",
    ".local/share/custom-scripts-rust",
    ".local/share/mlx-omarchy",
    ".local/share/nautilus-python",
    ".local/share/omarchy",
    ".local/state/omarchy/current",
    ".local/state/omarchy/settings",
    ".local/state/omarchy/toggles",
    ".local/state/omarchy/powerprofiles",
    ".local/state/omarchy/workspace-layouts",
    ".local/state/omarchy/asahi-mic-gain.json",
    "Wallpapers",
    "Projects/epl-stats",
    "Projects/manas-flights",
];
const SYSTEM_PATHS: &[&str] = &[
    "etc/os-release",
    "etc/locale.conf",
    "etc/locale.gen",
    "etc/vconsole.conf",
    "etc/localtime",
    "etc/environment",
    "etc/pacman.conf",
    "etc/pacman.d/mirrorlist",
    "etc/pacman.d/mirrorlist.asahi-alarm",
    "etc/pacman.d/hooks",
    "etc/fastfetch",
    "etc/fonts",
    "etc/gtk-3.0",
    "etc/sddm.conf",
    "etc/sddm.conf.d",
    "etc/plymouth",
    "etc/modprobe.d",
    "etc/modules-load.d",
    "etc/sysctl.d",
    "etc/udev/rules.d",
    "etc/systemd/system",
    "etc/systemd/user",
    "etc/systemd/logind.conf.d",
];
const EXCLUDES: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "venv",
    ".venv",
    "__pycache__",
    ".cache",
];

fn capture(dir: &Path, file: &str, program: &str, args: &[&str], required: bool) -> io::Result<()> {
    match Command::new(program).args(args).output() {
        Ok(out) if out.status.success() => fs::write(dir.join(file), out.stdout),
        result => {
            let reason = match result {
                Ok(out) => format!("{}: {}", out.status, String::from_utf8_lossy(&out.stderr)),
                Err(e) => e.to_string(),
            };
            if required {
                return Err(io::Error::other(format!("{program}: {reason}")));
            }
            let mut log = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join("unavailable.txt"))?;
            writeln!(log, "{file}: {program} {}: {reason}", args.join(" "))
        }
    }
}

fn archive(snapshot: &Path, label: &str, base: &Path, candidates: &[&str]) -> io::Result<()> {
    let paths: Vec<_> = candidates
        .iter()
        .filter(|p| base.join(p).symlink_metadata().is_ok())
        .copied()
        .collect();
    fs::write(
        snapshot.join(format!("{label}-sources.txt")),
        paths.join("\n") + "\n",
    )?;
    let list = snapshot.join(format!(".{label}-sources.nul"));
    let mut bytes = Vec::new();
    for path in &paths {
        bytes.extend_from_slice(Path::new(path).as_os_str().as_bytes());
        bytes.push(0);
    }
    fs::write(&list, bytes)?;
    let name = format!("{label}.tar.zst");
    let partial = snapshot.join(format!("{name}.partial"));
    let log = fs::File::create(snapshot.join(format!("{label}-tar.log")))?;
    let mut cmd = Command::new("tar");
    cmd.args(["--create", "--zstd", "--acls", "--xattrs", "--file"])
        .arg(&partial)
        .arg("--directory")
        .arg(base)
        .args(["--null", "--verbatim-files-from"]);
    for pattern in EXCLUDES {
        cmd.arg(format!("--exclude={pattern}"));
    }
    let status = cmd
        .arg("--files-from")
        .arg(&list)
        .stderr(Stdio::from(log))
        .status()?;
    fs::remove_file(list)?;
    if !status.success() {
        return Err(io::Error::other(format!("{label}: tar failed ({status}); see {label}-tar.log. Incomplete archive retained as .partial.")));
    }
    fs::rename(partial, snapshot.join(&name))?;
    let listing = fs::File::create(snapshot.join(format!("{label}-contents.txt")))?;
    let status = Command::new("tar")
        .current_dir(snapshot)
        .args(["--zstd", "-tf", &name])
        .stdout(Stdio::from(listing))
        .status()?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "{label}: archive verification failed"
        )));
    }
    println!("Verified {name}");
    Ok(())
}

fn run() -> io::Result<()> {
    let home = PathBuf::from(env::var_os("HOME").ok_or_else(|| io::Error::other("HOME is unset"))?);
    let timestamp = Command::new("date")
        .args(["-u", "+%Y-%m-%d_%H%M%S_UTC"])
        .output()?;
    if !timestamp.status.success() {
        return Err(io::Error::other("Cannot determine backup time"));
    }
    let root = env::current_dir()?.join("snapshots");
    fs::create_dir_all(&root)?;
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    let snapshot = root.join(String::from_utf8_lossy(&timestamp.stdout).trim());
    fs::create_dir(&snapshot)?;
    fs::set_permissions(&snapshot, fs::Permissions::from_mode(0o700))?;
    println!("Creating {}", snapshot.display());
    fs::write(
        snapshot.join("INCOMPLETE"),
        "Backup has not finished successfully.\n",
    )?;
    fs::write(
        snapshot.join("source-home.txt"),
        format!("{}\n", home.display()),
    )?;
    let meta = snapshot.join("metadata");
    fs::create_dir(&meta)?;
    for (file, program, args, required) in [
        ("packages-all.txt", "pacman", vec!["-Q"], true),
        ("packages-explicit.txt", "pacman", vec!["-Qqe"], true),
        ("packages-native.txt", "pacman", vec!["-Qqen"], true),
        ("packages-foreign.txt", "pacman", vec!["-Qqem"], false),
        ("system.txt", "uname", vec!["-a"], true),
        (
            "user-services.txt",
            "systemctl",
            vec!["--user", "--root=/", "list-unit-files", "--no-pager"],
            false,
        ),
        (
            "system-services.txt",
            "systemctl",
            vec!["--root=/", "list-unit-files", "--no-pager"],
            false,
        ),
        (
            "gnome-desktop.dconf",
            "dconf",
            vec!["dump", "/org/gnome/desktop/"],
            false,
        ),
        ("gtk.dconf", "dconf", vec!["dump", "/org/gtk/"], false),
    ] {
        capture(&meta, file, program, &args, required)?;
    }
    fs::copy(
        "/usr/share/omarchy/version",
        meta.join("omarchy-version.txt"),
    )?;
    archive(&snapshot, "home", &home, HOME_PATHS)?;
    archive(&snapshot, "system", Path::new("/"), SYSTEM_PATHS)?;
    let sums = Command::new("sha256sum")
        .current_dir(&snapshot)
        .args(["home.tar.zst", "system.tar.zst"])
        .output()?;
    if !sums.status.success() {
        return Err(io::Error::other("Checksum generation failed"));
    }
    fs::write(snapshot.join("SHA256SUMS"), sums.stdout)?;
    let status = Command::new("sha256sum")
        .current_dir(&snapshot)
        .args(["-c", "SHA256SUMS"])
        .status()?;
    if !status.success() {
        return Err(io::Error::other("Checksum verification failed"));
    }
    fs::remove_file(snapshot.join("INCOMPLETE"))?;
    println!("Backup complete: {}", snapshot.display());
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Backup failed: {e}");
        std::process::exit(1);
    }
}
