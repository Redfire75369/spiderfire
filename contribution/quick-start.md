# Quick start

## Contents

-   [Available modules](#available-modules)
-   [Help command](#help-command)
-   [Basic commands](#basic-commands)

### Available modules

-   [assert](https://github.com/Redfire75369/spiderfire/tree/master/modules/src/assert)
-   [fs](https://github.com/Redfire75369/spiderfire/tree/master/modules/src/fs)
-   [path](https://github.com/Redfire75369/spiderfire/tree/master/modules/src/path)
-   [url](https://github.com/Redfire75369/spiderfire/tree/master/modules/src/url)

### Help command

To use the help command execute `spiderfire.exe` with the -h or --help flag

```bash
./spiderfire.exe --help
```

### Basic commands

Start a javascript repl, exit by pressing `ctrl + c` twice

```bash
./spiderfire.exe repl
```

Evaluate an inline javascript expression

```bash
./spiderfire.exe eval console.log('Hello there')
```

Run a javascript file

```bash
./spiderfire.exe run <your-file.js>
```
