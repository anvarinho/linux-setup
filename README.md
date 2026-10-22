# Omarchy setup backup

Timestamped backups are in `snapshots/` and are included in Git. Each completed snapshot contains:

- `home.tar.zst`: Hyprland, Omarchy shell/plugins, Rust widget sources and installed binaries, terminals, shell/editor settings, user services, desktop launchers, fonts, application icons, the selected cursor, and active theme configuration. Includes local tools and their Rust sources, the local Omarchy source, and the EPL/Manas projects used by widget timers.
- `system.tar.zst`: selected system configuration, including pacman settings, mirrors, systemd units, Asahi hardware settings, fonts, locale, display manager settings, and Plymouth boot splash configuration. Paths are relative to `/`.
- `metadata/`: installed package versions, explicit/native/foreign package lists, offline user/system unit enablement inventory, selected desktop dconf settings, architecture, and Omarchy version. Optional failures appear in `unavailable.txt`.
- `SHA256SUMS`, archive contents, and source lists. `INCOMPLETE` indicates a failed or unfinished backup.

This backs up setup files, not the whole machine. Browser profiles, SSH keys, keyrings, agent credentials/history, clipboard history, installed applications/runtimes, and unrelated projects are outside its scope. Build caches (`target`, `node_modules`, Python virtual environments), Git history, and old Omarchy state backups are excluded. Existing backup files inside selected config folders are retained. Config files and scripts can contain private values. The compiled `backup` executable is ignored by Git; rebuild it from `backup.rs` when needed.

Cursor backups include the selected `Anvarinho-BoldBars` theme and its default-theme selector. Other cursor themes and `~/.local/share/cursors/` development files are excluded. Custom application icons in `hicolor/` and `ghostty-custom.png` are retained. If you switch cursor themes later, update the selected theme path in `backup.rs`.

Wallpaper images, videos, parallax assets, and theme `backgrounds/` directories are excluded from new backups, along with the active background symlink. Cached Omarchy installer packages are excluded because they embed stock wallpapers. Background plugin code and settings are retained. Older snapshots retain their original contents.

## Create another backup

Run from this folder:

```bash
rustc --edition=2021 backup.rs -o backup
./backup
```

Requires Rust, GNU tar, zstd, sha256sum, and pacman. The utility reads the live setup without changing it; it verifies archive readability and checksums before marking the backup complete. Databases included with the widget projects are live file copies, not transactional database backups.

## Verify and inspect

Change into the desired timestamped snapshot, then run:

```bash
sha256sum -c SHA256SUMS
tar --zstd -tf home.tar.zst
tar --zstd -tf system.tar.zst
```

## Restore

Use a compatible Omarchy installation on the same architecture first. See `metadata/omarchy-version.txt`, `metadata/system.txt`, and the package lists. This machine uses Asahi/ARM; review hardware-specific files before restoring elsewhere. Package lists record the installation; package availability and exact versions may differ later.

From the snapshot directory, extract into a staging directory for review:

```bash
mkdir -p restore/home restore/system
tar --zstd --acls --xattrs -xf home.tar.zst -C restore/home
tar --zstd --acls --xattrs -xf system.tar.zst -C restore/system
```

Review the staged files and back up your destination configuration. To restore the home setup, log out of the graphical desktop, use a TTY, then run from the snapshot directory:

```bash
tar --zstd --acls --xattrs --no-same-owner -xf home.tar.zst -C "$HOME"
systemctl --user daemon-reload
dconf load /org/gnome/desktop/ < metadata/gnome-desktop.dconf
dconf load /org/gtk/ < metadata/gtk.dconf
fc-cache -f
```

Only load dconf files that exist. Extraction overwrites matching files and does not delete newer files. The original home directory is recorded in `source-home.txt`; absolute symlinks, service paths, and some scripts refer to `/home/anvarinho` and need adjustment for another username. Restore selected system files from `restore/system/` with administrator privileges after review; copying the complete system archive onto another installation can overwrite service and hardware settings.

Reinstall the packages and external tools you need before logging back in. Some local wrappers and the Hermes service depend on applications outside this backup (for example Hermes, the Zen browser, and the CUA driver). Active theme configuration is included. Wallpapers are omitted; install and select a wallpaper separately after restoring, and supply parallax assets separately if you use them.
