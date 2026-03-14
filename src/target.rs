use std::fmt;

/// The target triple this binary was compiled for, captured at build time.
pub const TARGET: &str = env!("YS_TARGET");

/// The architecture component of a target triple (e.g., `aarch64`, `x86_64`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arch {
    Aarch64,
    X86_64,
    Arm,
    Armv7,
    Other,
}

impl Arch {
    fn parse(s: &str) -> Self {
        match s {
            "aarch64" => Arch::Aarch64,
            "x86_64" => Arch::X86_64,
            "arm" => Arch::Arm,
            "armv7" => Arch::Armv7,
            _ => Arch::Other,
        }
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arch::Aarch64 => write!(f, "aarch64"),
            Arch::X86_64 => write!(f, "x86_64"),
            Arch::Arm => write!(f, "arm"),
            Arch::Armv7 => write!(f, "armv7"),
            Arch::Other => write!(f, "unknown"),
        }
    }
}

/// The operating system component of a target triple (e.g., `darwin`, `linux`, `windows`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Os {
    Darwin,
    Linux,
    Windows,
    Other,
}

impl Os {
    fn parse(s: &str) -> Self {
        if s.contains("darwin") {
            Os::Darwin
        } else if s.contains("linux") {
            Os::Linux
        } else if s.contains("windows") {
            Os::Windows
        } else {
            Os::Other
        }
    }
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Os::Darwin => write!(f, "macOS"),
            Os::Linux => write!(f, "Linux"),
            Os::Windows => write!(f, "Windows"),
            Os::Other => write!(f, "unknown"),
        }
    }
}

/// A parsed Rust target triple (e.g., `aarch64-apple-darwin`).
///
/// Represents the architecture, vendor, and OS extracted from the full triple string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub triple: String,
    pub arch: Arch,
    pub os: Os,
}

impl Target {
    /// Parse a target triple string into its components.
    ///
    /// ```
    /// use yaml_schema::target::Target;
    /// use yaml_schema::target::Arch;
    /// use yaml_schema::target::Os;
    ///
    /// let target = Target::parse("aarch64-apple-darwin");
    /// assert_eq!(target.arch, Arch::Aarch64);
    /// assert_eq!(target.os, Os::Darwin);
    /// assert_eq!(target.triple, "aarch64-apple-darwin");
    /// ```
    pub fn parse(triple: &str) -> Self {
        let parts: Vec<&str> = triple.splitn(4, '-').collect();
        let arch = Arch::parse(parts.first().unwrap_or(&""));
        let os_str = parts.get(2).unwrap_or(&"");
        let os = Os::parse(os_str);

        Target {
            triple: triple.to_string(),
            arch,
            os,
        }
    }

    /// Returns the target triple this binary was compiled for.
    pub fn current() -> Self {
        Self::parse(TARGET)
    }

    /// Returns true if this target is a macOS (Darwin) target.
    pub fn is_macos(&self) -> bool {
        self.os == Os::Darwin
    }

    /// Returns true if this target is an Apple Silicon (aarch64 + Darwin) target.
    pub fn is_apple_silicon(&self) -> bool {
        self.arch == Arch::Aarch64 && self.os == Os::Darwin
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.triple)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_aarch64_apple_darwin() {
        let target = Target::parse("aarch64-apple-darwin");
        assert_eq!(target.arch, Arch::Aarch64);
        assert_eq!(target.os, Os::Darwin);
        assert_eq!(target.triple, "aarch64-apple-darwin");
        assert!(target.is_macos());
        assert!(target.is_apple_silicon());
    }

    #[test]
    fn test_parse_x86_64_apple_darwin() {
        let target = Target::parse("x86_64-apple-darwin");
        assert_eq!(target.arch, Arch::X86_64);
        assert_eq!(target.os, Os::Darwin);
        assert!(target.is_macos());
        assert!(!target.is_apple_silicon());
    }

    #[test]
    fn test_parse_x86_64_unknown_linux_gnu() {
        let target = Target::parse("x86_64-unknown-linux-gnu");
        assert_eq!(target.arch, Arch::X86_64);
        assert_eq!(target.os, Os::Linux);
        assert!(!target.is_macos());
    }

    #[test]
    fn test_parse_x86_64_pc_windows_msvc() {
        let target = Target::parse("x86_64-pc-windows-msvc");
        assert_eq!(target.arch, Arch::X86_64);
        assert_eq!(target.os, Os::Windows);
        assert!(!target.is_macos());
    }

    #[test]
    fn test_display() {
        let target = Target::parse("aarch64-apple-darwin");
        assert_eq!(format!("{target}"), "aarch64-apple-darwin");
    }

    #[test]
    fn test_current_target() {
        let target = Target::current();
        assert!(!target.triple.is_empty());
    }

    #[test]
    fn test_os_display() {
        assert_eq!(format!("{}", Os::Darwin), "macOS");
        assert_eq!(format!("{}", Os::Linux), "Linux");
        assert_eq!(format!("{}", Os::Windows), "Windows");
    }

    #[test]
    fn test_arch_display() {
        assert_eq!(format!("{}", Arch::Aarch64), "aarch64");
        assert_eq!(format!("{}", Arch::X86_64), "x86_64");
    }
}
