# Build Instructions

Below are the instructions on how you can build Spiderfire.
All instructions here assume you have installed `rustup`, `rustc` and `cargo`. Refer to https://rustup.rs/ for installation instructions.

## Contents

- [Windows](#windows)
	- [MSVC](#msvc)
	- [GNU](#gnu)
- [MacOS](#macos)
- [Linux](#linux)
	- [Debian/Ubuntu](#debian-ubuntu-and-derivatives)
	- [Fedora/RHEL](#fedora-rhel-and-derivatives)
	- [OpenSUSE](#opensuse-and-derivatives)
	- [Arch Linux](#arch-linux-and-derivatives)
	- [Gentoo Linux](#gentoo-linux-and-derivatives)
	- [Alpine Linux](#alpine-linux)
- [BSD](#bsd)
	- [FreeBSD](#freebsd)

## Windows

### MSVC

1. Follow the instructions at [Windows Prerequisites](https://firefox-source-docs.mozilla.org/setup/windows_build.html) (Steps 1.1 and 1.2)
2. Start Visual Studio Developer Command Prompt

```batch
"C:\Program Files (x86)\Microsoft Visual Studio\2017\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
```

3. Download and Install Clang for Windows (64 bit) from [LLVM Releases](https://releases.llvm.org/download.html)
4. Set Environment Variables

```batch
build.bat
```

5. Build with Cargo

```batch
cargo build
```

### GNU

-- TODO --

## MacOS

-- TODO --

## Linux

### Debian, Ubuntu and Derivatives

1. Install Build Dependencies

```shell
sudo apt -y install python python3 autoconf2.13 gcc g++ make clang llvm pkg-config libzlibg1-dev
```

2. Build with Cargo

```shell
cargo build
```

### Fedora, RHEL and Derivatives

-- TODO --

### OpenSUSE and Derivatives

-- TODO --

### Arch Linux and Derivatives

-- TODO --

### Gentoo Linux and Derivatives

-- TODO --

### Alpine Linux

-- TODO --

## *BSD

### FreeBSD

-- TODO --

