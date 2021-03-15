[![Discord Server](https://img.shields.io/discord/579354150639370348?color=7389D8&label=Discord&labelColor=6A7EC2&logo=discord&logoColor=FFFFFF&style=flat-square)](https://discord.gg/RyQwwzW)
[![License: MPL 2.0](https://img.shields.io/static/v1?color=7389D8&label=License&labelColor=5D5D5D&message=MPL%202.0&color=4DC71F&style=flat-square)](https://choosealicense.com/licenses/mpl-2.0/)

# Spiderfire
Spiderfire is a javascript runtime built with Mozilla's SpiderMonkey engine and Rust.

Spiderfire aims to disrupt the server-side javascript runtime environment.


### Build Instructions
1. Follow Instructions [here](https://github.com/servo/mozjs/blob/master/README.md)
2. Run `cargo build`

#### Debian-based Linux
1. Install Build Prerequisites:
```shell
sudo apt-get install clang-6.0 autoconf2.13
```
2. Build with Cargo
```shell
cargo build
```

#### Windows
1. Follow the directions at [Windows Prerequisites](https://developer.mozilla.org/en-US/docs/Mozilla/Developer_guide/Build_Instructions/Windows_Prerequisites)
2. Start Visual Studio Developer Command Prompt
```batch
"C:\Program Files (x86)\Microsoft Visual Studio\2017\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
```
3. Download and install Clang for Windows (64 bit) from [LLVM Releases](https://releases.llvm.org/download.html)
4. Set Environment Variables
```batch
build.bat
```
5. Build with Cargo
```batch
cargo build
```
