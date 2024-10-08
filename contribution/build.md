# Build Instructions

Below are the instructions on how you can build Spiderfire.
All instructions here assume you have installed `rustup`, `rustc` and `cargo`.
Refer to https://rustup.rs/ for installation instructions.

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
- [FreeBSD](#freebsd)

## Windows

### MSVC

1. Download and unzip [MozTools 4.0] to `C:\moztools-4.0`.
2. Download and install LLVM for Windows (64 bit) from [LLVM Releases].
	- Note: When installing LLVM, choose to add LLVM to the system path.

3. Follow the instructions at [Just Installation].

4. Set Environment Variables <br/>
   Powershell:
   ```powershell
   .\build.ps1
   ```
   Command Prompt:
   ```batch
   build.bat
   ```

5. Build with Cargo
   ```powershell
   just build
   ```

[MozTools 4.0]: https://github.com/servo/servo-build-deps/releases/download/msvc-deps/moztools-4.0.zip

[LLVM Releases]: https://github.com/llvm/llvm-project/releases/latest

## MacOS

1. Install Xcode command line tools, if you haven't.
   ```shell
   xcode-select --install
   ```

2. Install Build Dependencies.
   ```shell
   brew install python3 llvm pkg-config make just
   ```

3. Build with Cargo
   ```shell
   CC=clang CXX=clang++ just build
   ```

## Linux

### Debian, Ubuntu and Derivatives

1. Install Build Dependencies.
   ```shell
   sudo apt install -y python3 python3-distutils autoconf2.13 clang llvm make pkg-config zlib1g-dev
   ```

2. Follow the instructions at [Just Installation].

3. Build with Cargo
   ```shell
   CC=clang CXX=clang++ just build
   ```

[Just Installation]: https://github.com/casey/just#installation
