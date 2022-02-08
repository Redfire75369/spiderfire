# Quick Start

## Contents

- [Available modules](#available-modules)
- [Basic commands](#basic-commands)

### Available Modules

- [assert](https://github.com/Redfire75369/spiderfire/tree/master/modules/src/assert)
- [fs](https://github.com/Redfire75369/spiderfire/tree/master/modules/src/fs)
- [path](https://github.com/Redfire75369/spiderfire/tree/master/modules/src/path)
- [url](https://github.com/Redfire75369/spiderfire/tree/master/modules/src/url)

### Basic Commands

Get a list of available subcommands and flags.

```bash
# windows
./spiderfire.exe --help

# linux
./spiderfire --help
```

Start a Javascript repl, exit by pressing `Ctrl + C` twice.

```bash
# windows
./spiderfire.exe repl

# linux
./spiderfire repl
```

Evaluate an inline Javascript expression.

```bash
# windows
./spiderfire.exe eval console.log('Hello there');

# linux
./spiderfire eval console.log('Hello there');
```

Run a Javascript file.

```bash
# windows
./spiderfire.exe run <your-file.js>

# linux
./spiderfire run <your-file.js>
```
