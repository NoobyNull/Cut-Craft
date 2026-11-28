//! What computer PowerWhittle is running on, for bug and crash reports: the operating system by
//! its everyday name and exact version (macOS 14.5 Sonoma, Windows 11 Pro 23H2 build
//! 22631.4460, Debian GNU/Linux 12), whether the processor is Intel/AMD or ARM, and the
//! graphics card and how it is drawn to. Someone reporting a bug rarely knows these, and a
//! fix often depends on them.
//!
//! Nothing personal is read: not the computer's name, the user name or any folder. Each
//! system is read the one way it says itself plainly: `/etc/os-release` on Linux, `sw_vers`
//! on a Mac, and the registry's `CurrentVersion` key on Windows. That key still calls
//! Windows 11 "Windows 10", so the build number decides: 22000 and later is Windows 11.

use std::fmt::Write as _;
use std::sync::OnceLock;

/// The longest [`System::short`]: what the crash-report service takes.
pub const SHORT_CHARS: usize = 40;
/// The first Windows 11 build: Windows says "10" for both, so the build decides.
const WINDOWS_11_BUILD: u32 = 22_000;
/// The first Windows 10 build.
const WINDOWS_10_BUILD: u32 = 10_240;

/// The computer PowerWhittle is running on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct System {
    /// The operating system and its release, short: "Windows 11 23H2", "macOS 14 Sonoma",
    /// "Ubuntu 24.04". For grouping reports.
    pub os: String,
    /// The same, exactly: "Windows 11 Pro 23H2 (build 22631.4460)",
    /// "macOS 14.5 Sonoma (build 23F79)", "Ubuntu 24.04.1 LTS (Noble Numbat)".
    pub os_exact: String,
    /// The processor family PowerWhittle was built for: `x86_64` or `aarch64`.
    pub arch: String,
    /// More that matters on this system, each "what: value": the Linux kernel and desktop
    /// session (Wayland or X11), or that a Mac runs the Intel build through Rosetta.
    pub more: Vec<String>,
}

impl System {
    /// One short line for grouping crashes: `Windows 11 23H2 x86_64`. Plain ASCII, at most
    /// [`SHORT_CHARS`] characters.
    pub fn short(&self) -> String {
        let text: String = format!("{} {}", self.os, self.arch)
            .chars()
            .filter(|c| c.is_ascii_graphic() || *c == ' ')
            .collect();
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        text.chars()
            .take(SHORT_CHARS)
            .collect::<String>()
            .trim()
            .to_owned()
    }

    /// The lines a report carries: the system exactly, the processor in plain words, then
    /// anything more.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("System: {}", self.os_exact),
            format!("Processor: {}", arch_words(&self.arch)),
        ];
        lines.extend(self.more.iter().cloned());
        lines
    }
}

/// `arch` with what it means: `x86_64 (Intel or AMD, 64-bit)`.
pub fn arch_words(arch: &str) -> String {
    let words = match arch {
        "x86_64" => "Intel or AMD, 64-bit",
        "x86" => "Intel or AMD, 32-bit",
        "aarch64" => "ARM, 64-bit",
        "arm" => "ARM, 32-bit",
        "riscv64" => "RISC-V, 64-bit",
        _ => return arch.to_owned(),
    };
    format!("{arch} ({words})")
}

static THIS: OnceLock<System> = OnceLock::new();
static GRAPHICS: OnceLock<String> = OnceLock::new();

/// This computer, read once.
pub fn this() -> &'static System {
    THIS.get_or_init(detect)
}

/// This computer if it has been read already: what a crash, which must not wait, can say.
pub fn this_if_read() -> Option<&'static System> {
    THIS.get()
}

/// Says which graphics card draws PowerWhittle and how, once it is known at startup.
pub fn set_graphics(name: &str, backend: &str, driver: &str, driver_info: &str) {
    let _ = GRAPHICS.set(graphics_line(name, backend, driver, driver_info));
}

/// The graphics card, as [`set_graphics`] was told it.
pub fn graphics() -> Option<&'static str> {
    GRAPHICS.get().map(String::as_str)
}

/// "AMD Radeon RX 9070 XT (Vulkan, driver radv 25.1)": blanks left out.
fn graphics_line(name: &str, backend: &str, driver: &str, driver_info: &str) -> String {
    let name = if name.trim().is_empty() {
        "an unnamed graphics card"
    } else {
        name.trim()
    };
    let driver = [driver.trim(), driver_info.trim()]
        .into_iter()
        .filter(|d| !d.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let how = if driver.is_empty() {
        backend.to_owned()
    } else {
        format!("{backend}, driver {driver}")
    };
    format!("{name} ({how})")
}

/// Every line a report carries about the computer: [`System::lines`] then the graphics.
pub fn report_lines() -> Vec<String> {
    let mut lines = this().lines();
    lines.push(format!(
        "Graphics: {}",
        graphics().unwrap_or("not started yet")
    ));
    lines
}

/// Crash log `crash` without the details of the computer: its first line names only the
/// kind of system (`linux`, `windows`, `macos`), and the `System:` lines are taken out. What
/// goes when the person has not said the details may go too.
pub fn without_details(crash: &str) -> String {
    let (first, rest) = crash.split_once('\n').unwrap_or((crash, ""));
    let first = match (first.split_once(" on "), first.split_once(" crashed: ")) {
        (Some((head, _)), Some((_, panic))) if crate::logs::after_app_name(head).is_some() => {
            format!("{head} on {} crashed: {panic}", std::env::consts::OS)
        }
        _ => first.to_owned(),
    };
    let mut out = first;
    out.push('\n');
    let mut rest = rest;
    while let Some(at) = rest.find("\n\nSystem: ") {
        out.push_str(&rest[..at + 2]);
        let block = &rest[at + 2..];
        rest = block.find("\n\n").map_or("", |end| &block[end + 2..]);
    }
    out.push_str(rest);
    out
}

fn detect() -> System {
    let arch = std::env::consts::ARCH.to_owned();
    #[cfg(target_os = "linux")]
    {
        let release = std::fs::read_to_string("/etc/os-release")
            .or_else(|_| std::fs::read_to_string("/usr/lib/os-release"))
            .unwrap_or_default();
        let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease").ok();
        let session = std::env::var("XDG_SESSION_TYPE").ok();
        linux(&release, kernel.as_deref(), session.as_deref(), arch)
    }
    #[cfg(target_os = "macos")]
    {
        let ask = |args: &[&str]| {
            std::process::Command::new(args[0])
                .args(&args[1..])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        };
        let version = ask(&["sw_vers", "-productVersion"]).unwrap_or_default();
        let build = ask(&["sw_vers", "-buildVersion"]);
        let rosetta = ask(&["sysctl", "-n", "sysctl.proc_translated"]).as_deref() == Some("1");
        macos(&version, build.as_deref(), rosetta, arch)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        /// Runs `reg` without a console window flashing up.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let key = std::process::Command::new("reg")
            .args([
                "query",
                r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()
            .map_or_default(|o| String::from_utf8_lossy(&o.stdout).into_owned());
        windows(&key, arch)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        System {
            os: std::env::consts::OS.to_owned(),
            os_exact: std::env::consts::OS.to_owned(),
            arch,
            more: vec![],
        }
    }
}

/// A `KEY=value` file's value for `key`, its quotes taken off (`/etc/os-release`).
fn value_of<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .filter_map(|l| l.trim().split_once('='))
        .find(|(k, _)| k.trim() == key)
        .map(|(_, v)| v.trim().trim_matches(|c| c == '"' || c == '\''))
        .filter(|v| !v.is_empty())
}

/// A Linux system from its `os-release` file, its kernel (`/proc/sys/kernel/osrelease`) and
/// its desktop session (`XDG_SESSION_TYPE`).
pub fn linux(release: &str, kernel: Option<&str>, session: Option<&str>, arch: String) -> System {
    let name = value_of(release, "NAME").unwrap_or("Linux");
    let os = match value_of(release, "VERSION_ID") {
        Some(version) => format!("{name} {version}"),
        None => name.to_owned(),
    };
    let os_exact = value_of(release, "PRETTY_NAME")
        .map(str::to_owned)
        .or_else(|| value_of(release, "VERSION").map(|v| format!("{name} {v}")))
        .unwrap_or_else(|| os.clone());
    let mut more = vec![];
    if let Some(kernel) = kernel.map(str::trim).filter(|k| !k.is_empty()) {
        more.push(format!("Linux kernel: {kernel}"));
    }
    match session.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("wayland") => more.push("Desktop: Wayland".to_owned()),
        Some("x11") => more.push("Desktop: X11".to_owned()),
        Some("tty" | "") | None => {}
        Some(other) => more.push(format!("Desktop: {other}")),
    }
    System {
        os,
        os_exact,
        arch,
        more,
    }
}

/// The name each macOS release goes by, from its version (`10.7` is Lion, `14` Sonoma).
/// Releases since 11 are named by their first number; before that, by the second.
pub fn macos_name(version: &str) -> Option<&'static str> {
    let mut parts = version.trim().split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next().and_then(|m| m.parse().ok()).unwrap_or(0);
    Some(match (major, minor) {
        (10, 0) => "Cheetah",
        (10, 1) => "Puma",
        (10, 2) => "Jaguar",
        (10, 3) => "Panther",
        (10, 4) => "Tiger",
        (10, 5) => "Leopard",
        (10, 6) => "Snow Leopard",
        (10, 7) => "Lion",
        (10, 8) => "Mountain Lion",
        (10, 9) => "Mavericks",
        (10, 10) => "Yosemite",
        (10, 11) => "El Capitan",
        (10, 12) => "Sierra",
        (10, 13) => "High Sierra",
        (10, 14) => "Mojave",
        (10, 15) => "Catalina",
        // Big Sur tells older programs it is 10.16
        (10, 16) | (11, _) => "Big Sur",
        (12, _) => "Monterey",
        (13, _) => "Ventura",
        (14, _) => "Sonoma",
        (15, _) => "Sequoia",
        // Tahoe tells older programs it is 16
        (16 | 26, _) => "Tahoe",
        _ => return None,
    })
}

/// A Mac from `sw_vers`: its version (`14.5`) and build (`23F79`), and whether this, the
/// Intel build, runs through Rosetta on an Apple processor.
pub fn macos(version: &str, build: Option<&str>, rosetta: bool, arch: String) -> System {
    let version = version.trim();
    if version.is_empty() {
        return System {
            os: "macOS".to_owned(),
            os_exact: "macOS (version unknown)".to_owned(),
            arch,
            more: vec![],
        };
    }
    let mut parts = version.split('.');
    let major = parts.next().unwrap_or_default();
    // 10.x releases are told apart by their second number, later ones by their first
    let release = if major == "10" {
        format!("10.{}", parts.next().unwrap_or("0"))
    } else {
        major.to_owned()
    };
    let named = |v: &str| match macos_name(version) {
        Some(name) => format!("macOS {v} {name}"),
        None => format!("macOS {v}"),
    };
    let os_exact = match build.map(str::trim).filter(|b| !b.is_empty()) {
        Some(build) => format!("{} (build {build})", named(version)),
        None => named(version),
    };
    let more = if rosetta {
        vec!["Runs through: Rosetta (the Intel build on an Apple processor)".to_owned()]
    } else {
        vec![]
    };
    System {
        os: named(&release),
        os_exact,
        arch,
        more,
    }
}

/// A value from `reg query`'s answer: lines like `    CurrentBuild    REG_SZ    22631`.
fn reg_value<'a>(key: &'a str, name: &str) -> Option<&'a str> {
    key.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        if words.next()? != name {
            return None;
        }
        let _kind = words.next()?;
        let rest = line.split_once("REG_")?.1;
        let value = rest.split_once(char::is_whitespace)?.1.trim();
        (!value.is_empty()).then_some(value)
    })
}

/// A number `reg query` gives, in hex (`0x1234`) or not.
fn reg_number(value: &str) -> Option<u32> {
    match value.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => value.parse().ok(),
    }
}

/// Windows from `reg query HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`. Its
/// `ProductName` says "Windows 10" on Windows 11 too, so the build number names it.
pub fn windows(key: &str, arch: String) -> System {
    let build = reg_value(key, "CurrentBuild")
        .or_else(|| reg_value(key, "CurrentBuildNumber"))
        .and_then(reg_number);
    let product = reg_value(key, "ProductName").unwrap_or("Windows");
    let name = match build {
        Some(b) if b >= WINDOWS_11_BUILD => "Windows 11".to_owned(),
        Some(b) if b >= WINDOWS_10_BUILD => "Windows 10".to_owned(),
        // Windows 8.1 and before name themselves rightly
        _ => product
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" "),
    };
    // "Windows 10 Pro" -> "Pro": the edition, whatever number the name has
    let edition = product
        .strip_prefix("Windows ")
        .and_then(|rest| rest.split_once(' '))
        .map(|(_, edition)| edition.trim())
        .filter(|e| !e.is_empty());
    // DisplayVersion ("23H2") since 20H2; ReleaseId ("1909") before
    let release = reg_value(key, "DisplayVersion").or_else(|| reg_value(key, "ReleaseId"));
    let os = match release {
        Some(release) => format!("{name} {release}"),
        None => name.clone(),
    };
    let mut exact = name;
    for part in [edition, release].into_iter().flatten() {
        exact.push(' ');
        exact.push_str(part);
    }
    if let Some(build) = build {
        let update = reg_value(key, "UBR").and_then(reg_number);
        let _ = match update {
            Some(update) => write!(exact, " (build {build}.{update})"),
            None => write!(exact, " (build {build})"),
        };
    }
    System {
        os,
        os_exact: exact,
        arch,
        more: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const X64: fn() -> String = || "x86_64".to_owned();

    #[test]
    fn windows_11_is_told_by_its_build_though_it_calls_itself_10() {
        let key = r"
HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion
    SystemRoot    REG_SZ    C:\WINDOWS
    CurrentBuild    REG_SZ    22631
    CurrentBuildNumber    REG_SZ    22631
    DisplayVersion    REG_SZ    23H2
    EditionID    REG_SZ    Professional
    ProductName    REG_SZ    Windows 10 Pro
    ReleaseId    REG_SZ    2009
    UBR    REG_DWORD    0x116c
    RegisteredOwner    REG_SZ    Sam Person
";
        let s = windows(key, X64());
        assert_eq!(s.os, "Windows 11 23H2");
        assert_eq!(s.os_exact, "Windows 11 Pro 23H2 (build 22631.4460)");
        assert_eq!(s.short(), "Windows 11 23H2 x86_64");
        assert!(
            !s.lines().join("\n").contains("Sam"),
            "the registered owner is never read"
        );
    }

    #[test]
    fn windows_10_keeps_its_name_and_release() {
        let key = "    ProductName    REG_SZ    Windows 10 Home\n    CurrentBuild    REG_SZ    \
                   19045\n    DisplayVersion    REG_SZ    22H2\n    UBR    REG_DWORD    0x1393\n";
        let s = windows(key, X64());
        assert_eq!(s.os, "Windows 10 22H2");
        assert_eq!(s.os_exact, "Windows 10 Home 22H2 (build 19045.5011)");
    }

    #[test]
    fn older_windows_use_release_id_or_their_own_name() {
        let key = "    ProductName    REG_SZ    Windows 10 Pro\n    CurrentBuild    REG_SZ    \
                   18363\n    ReleaseId    REG_SZ    1909\n";
        assert_eq!(windows(key, X64()).os, "Windows 10 1909");
        let key =
            "    ProductName    REG_SZ    Windows 8.1 Pro\n    CurrentBuild    REG_SZ    9600\n";
        let s = windows(key, X64());
        assert_eq!(s.os, "Windows 8.1");
        assert_eq!(s.os_exact, "Windows 8.1 Pro (build 9600)");
        assert_eq!(
            windows("", X64()).os,
            "Windows",
            "nothing read: still Windows"
        );
    }

    #[test]
    fn every_mac_release_has_its_name() {
        assert_eq!(macos_name("10.7.5"), Some("Lion"));
        assert_eq!(macos_name("10.15"), Some("Catalina"));
        assert_eq!(macos_name("10.16"), Some("Big Sur"));
        assert_eq!(macos_name("11.7.10"), Some("Big Sur"));
        assert_eq!(macos_name("14.5"), Some("Sonoma"));
        assert_eq!(macos_name("15.1"), Some("Sequoia"));
        assert_eq!(macos_name("26.0"), Some("Tahoe"));
        assert_eq!(macos_name("99.0"), None);
        let s = macos("14.5", Some("23F79"), false, "aarch64".into());
        assert_eq!(s.os, "macOS 14 Sonoma");
        assert_eq!(s.os_exact, "macOS 14.5 Sonoma (build 23F79)");
        assert_eq!(s.short(), "macOS 14 Sonoma aarch64");
        let lion = macos("10.7.5", None, false, X64());
        assert_eq!(lion.os, "macOS 10.7 Lion");
        assert_eq!(lion.os_exact, "macOS 10.7.5 Lion");
    }

    #[test]
    fn a_mac_running_the_intel_build_through_rosetta_says_so() {
        let s = macos("15.0", None, true, X64());
        assert!(s.lines().iter().any(|l| l.contains("Rosetta")));
        assert_eq!(macos("", None, false, X64()).os, "macOS");
    }

    #[test]
    fn linux_names_its_distribution_kernel_and_desktop() {
        let debian = "PRETTY_NAME=\"Debian GNU/Linux 12 (bookworm)\"\nNAME=\"Debian GNU/Linux\"\n\
                      VERSION_ID=\"12\"\nVERSION=\"12 (bookworm)\"\nID=debian\n";
        let s = linux(debian, Some("6.1.0-26-amd64\n"), Some("wayland"), X64());
        assert_eq!(s.os, "Debian GNU/Linux 12");
        assert_eq!(s.os_exact, "Debian GNU/Linux 12 (bookworm)");
        assert_eq!(
            s.lines(),
            [
                "System: Debian GNU/Linux 12 (bookworm)",
                "Processor: x86_64 (Intel or AMD, 64-bit)",
                "Linux kernel: 6.1.0-26-amd64",
                "Desktop: Wayland",
            ]
        );
        // rolling releases have no version
        let arch = "NAME=\"Arch Linux\"\nPRETTY_NAME=\"Arch Linux\"\nID=arch\nBUILD_ID=rolling\n";
        let s = linux(arch, None, Some("x11"), "aarch64".into());
        assert_eq!(s.short(), "Arch Linux aarch64");
        assert!(
            s.lines()
                .contains(&"Processor: aarch64 (ARM, 64-bit)".to_owned())
        );
        assert!(s.lines().contains(&"Desktop: X11".to_owned()));
        let ubuntu = "NAME='Ubuntu'\nVERSION_ID='24.04'\nVERSION='24.04.1 LTS (Noble Numbat)'\n";
        let s = linux(ubuntu, None, None, X64());
        assert_eq!(s.os, "Ubuntu 24.04");
        assert_eq!(s.os_exact, "Ubuntu 24.04.1 LTS (Noble Numbat)");
        assert_eq!(linux("", None, None, X64()).os, "Linux", "no os-release");
    }

    #[test]
    fn the_short_line_fits_the_service() {
        let long =
            "NAME=\"A Very Long Distribution Name — Special Édition\"\nVERSION_ID=\"2026.11.17\"\n";
        let short = linux(long, None, None, X64()).short();
        assert!(short.chars().count() <= SHORT_CHARS, "{short}");
        assert!(
            short.chars().all(|c| c == ' ' || c.is_ascii_graphic()),
            "{short}"
        );
    }

    #[test]
    fn graphics_name_the_card_and_how_it_is_drawn_to() {
        assert_eq!(
            graphics_line("AMD Radeon RX 9070 XT", "Vulkan", "radv", "Mesa 25.1"),
            "AMD Radeon RX 9070 XT (Vulkan, driver radv Mesa 25.1)"
        );
        assert_eq!(
            graphics_line("", "Gl", "", ""),
            "an unnamed graphics card (Gl)"
        );
    }

    #[test]
    fn a_crash_log_can_go_without_the_details_of_the_computer() {
        let crash = "PowerWhittle 0.6.16 on Windows 11 23H2 x86_64 crashed: panicked at a.rs:1:1:\n\
                     boom\n\nBacktrace:\n  0: here\n\nSystem: Windows 11 Pro 23H2 (build \
                     22631.4460)\nProcessor: x86_64\nGraphics: Card (Dx12)\n\nLog:\nline\n";
        let plain = without_details(crash);
        assert_eq!(
            plain,
            format!(
                "PowerWhittle 0.6.16 on {} crashed: panicked at a.rs:1:1:\nboom\n\nBacktrace:\n  \
                 0: here\n\nLog:\nline\n",
                std::env::consts::OS
            )
        );
        assert_eq!(without_details(&plain), plain, "nothing more to take out");
    }

    #[test]
    fn this_computer_reads_as_something() {
        let s = this();
        assert_ne!(s.os, "");
        assert_eq!(s.arch, std::env::consts::ARCH);
        assert!(report_lines()[0].starts_with("System: "));
    }
}
