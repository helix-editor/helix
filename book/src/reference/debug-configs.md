# Debugger configurations

This page provides additional debugger configurations beyond the default ones included with Helix, which can be found in [languages.toml](https://github.com/helix-editor/helix/blob/master/languages.toml).

## lldb-vscode

> 💡`lldb-vscode` has recently changed its name to `lldb-dap`. You can find the source code [here](https://github.com/llvm/llvm-project/tree/main/lldb/tools/lldb-dap). If you are using the Helix master branch, since [#10091](https://github.com/helix-editor/helix/commit/b24c465a086b60096d7e95852d4d2db84f5ccbd3) `lldb-dap` is used

Helix natively supports debugging Rust, Zig, C, and C++ with `lldb-vscode`, which is part of `llvm/lldb`.

### Installation Instructions

#### For macOS Users

1. Install LLVM using Homebrew:
   ```sh
   brew install llvm
   ```
2. Add LLVM to your `PATH` by including `$(brew --prefix)/opt/llvm/bin` in your `~/.bashrc` or `~/.zshrc` file.
3. Restart your shell to apply the changes.

#### For Debian 12 Users

1. Install lldb-15:
   ```sh
   sudo apt install lldb-15
   ```
2. Navigate to the directory containing `lldb-vscode-15`:
   ```sh
   cd $(dirname $(which lldb-vscode-15))
   ```
3. Create a symbolic link for `lldb-vscode` (or since [#10091](https://github.com/helix-editor/helix/commit/b24c465a086b60096d7e95852d4d2db84f5ccbd3) `lldb-dap`):
   ```sh
   sudo ln -s lldb-vscode-15 lldb-dap
   ```

#### For Arch Linux users

`pacman -S lldb`

#### For Fedora Users

```sh
sudo dnf install lldb
```

#### For General Linux Users: Installing from Official Pre-compiled Binaries

1. Download the latest pre-compiled LLVM binaries for Linux from the [releases page](https://github.com/llvm/llvm-project/releases). For example, at the time of writing, the latest version is available at:
   ```
   https://github.com/llvm/llvm-project/releases/download/llvmorg-17.0.6/clang+llvm-17.0.6-x86_64-linux-gnu-ubuntu-22.04.tar.xz
   ```
2. Unpack the binaries to a directory within your system's `$PATH`, such as `~/bin`, and create a symbolic link to the `lldb-vscode` executable located in the `bin` directory.

![Screenshot depicting the unpacking process](https://user-images.githubusercontent.com/12832280/195423210-fea5970c-9453-4a8d-8acc-b0cfd5d626e6.png)

### Configuration for Rust

An issue exists where [string variables display as memory addresses instead of their actual values](https://github.com/helix-editor/helix/issues/7007). To address this, `rust-lldb` offers a workaround by running specific lldb commands before loading the program. The paths may differ based on your Rust installation:

```sh
rust-lldb target/debug/hello_cargo
(lldb) command script import "/opt/homebrew/Cellar/rust/1.71.0/lib/rustlib/etc/lldb_lookup.py"
(lldb) command source -s 0 '/opt/homebrew/Cellar/rust/1.71.0/lib/rustlib/etc/lldb_commands'
```

By executing these commands, the debugger can display the contents of local string variables as shown below:

```lldb
(lldb) b src/main.rs:5
Breakpoint 1: where = hello_cargo`hello_cargo::main::h24135e338b19c0c6 + 212 at main.rs:5:5, address = 0x0000000100003cc0
(lldb) run
Process 62497 launched: '/path/to/hello_cargo/target/debug/hello_cargo' (arm64)
Process 62497 stopped
* thread #1, name = 'main', queue = 'com.apple.main-thread', stop reason = breakpoint 1.1
    frame #0: 0x0000000100003cc0 hello_cargo`hello_cargo::main::h24135e338b19c0c6 at main.rs:5:5
   2        let s = "world";
   3        let x = 2;
   4        let b = true;
-> 5        println!("Hello, {} {} {}!", s, x, b);
   6    }
(lldb) frame variable
(&str) s = "world" {
  data_ptr = 0x0000000100039d70 "worldHello,  !\n"
  length = 5
}
(int) x = 2
(bool) b = true
(lldb)
```

For `lldb-vscode`, replicate this functionality using [initCommands](https://github.com/llvm/llvm-project/tree/main/lldb/tools/lldb-vscode#launch-configuration-settings). Save the following Python snippet as `/usr/local/etc/lldb_vscode_rustc_primer.py` (create the directory if necessary):

```python
import subprocess
import pathlib
import lldb

## Determine the sysroot for the active Rust interpreter
rustlib_etc = pathlib.Path(subprocess.getoutput('rustc --print sysroot')) / 'lib' / 'rustlib' / 'etc'
if not rustlib_etc.exists():
    raise RuntimeError('Unable to determine rustc sysroot')

## Load lldb_lookup.py and execute lldb_commands with the correct path
lldb.debugger.HandleCommand(f"""command script import "{rustlib_etc / 'lldb_lookup.py'}" """)
lldb.debugger.HandleCommand(f"""command source -s 0 "{rustlib_etc / 'lldb_commands'}" """)
```

This script locates your active Rust installation and executes the lldb setup scripts as `rust-lldb` would. Update your `~/.config/helix/languages.toml` accordingly:

```toml
[[language]]
name = "rust"

[language.debugger]
name = "lldb-vscode"
transport = "stdio"
command = "lldb-vscode"

[[language.debugger.templates]]
name = "binary"
request = "launch"
completion = [ { name = "binary", completion = "filename" } ]
args = { program = "{0}", initCommands = [ "command script import /usr/local/etc/lldb_vscode_rustc_primer.py" ] }
```

## Codelldb

### Installation Instructions for Linux (Tested on Ubuntu)

1. Navigate to your home directory:
   ```sh
   cd ~
   ```
2. Determine your CPU's architecture to download the correct version of `codelldb`:
   ```sh
   lscpu | grep -oP 'Architecture:\s*\K.+'
   ```
3. Create a `bin` directory and access it:
   ```sh
   mkdir bin && cd bin
   ```
4. Use `curl` to download `codelldb`:
   ```sh
   sudo curl -L "https://github.com/vadimcn/vscode-lldb/releases/download/v1.7.0/codelldb-x86_64-linux.vsix" -o "codelldb-x86_64-linux.zip"
   ```
5. Unzip only the required folders (`extension/adapter` and `extension/lldb`):
   ```sh
   unzip "codelldb-x86_64-linux.zip" "extension/adapter/*" "extension/lldb/*"
   ```
6. Rename the `extension/` directory to `codelldb_adapter/`:
   ```sh
   mv extension/ codelldb_adapter
   ```
7. Remove the downloaded zip file:
   ```sh
   sudo rm "codelldb-x86_64-linux.zip"
   ```
8. Create a symbolic link from `codelldb_adapter/adapter/codelldb` to `/usr/bin/codelldb`:
   ```sh
   ln -s $(pwd)/codelldb_adapter/adapter/codelldb /usr/bin/codelldb
   ```

Test the installation by running `codelldb -h`.

### Configuration for Rust

You can also use `vscode-lldb`'s `codelldb` adapter for debugging Rust:

```toml
[[language]]
name = "rust"

[language.debugger]
command = "codelldb"
name = "codelldb"
port-arg = "--port {}"
transport = "tcp"

[[language.debugger.templates]]
name = "binary"
request = "launch"
[[language.debugger.templates.completion]]
completion = "filename"
name = "binary"

[language.debugger.templates.args]
program = "{0}"
runInTerminal = true
```

Test with `debug-start binary target/debug/zellij`, for example. Starting/stopping debugging and setting breakpoints should work as expected.

### Additional Information for Users Unable to Attach to a Running Process

If you're on Linux and unable to attach to a running process due to an "Operation not permitted" error, check if `ptrace` is blocking you. Consult this [Microsoft troubleshooting guide](https://github.com/Microsoft/MIEngine/wiki/Troubleshoot-attaching-to-processes-using-GDB) for solutions.

Possible solutions include:

1. Setting the value of `/proc/sys/kernel/yama/ptrace_scope` to `0` rather than `1`.
2. If the file is not present or Yama is not used, use `libcap2-bin` to grant `ptrace` permissions to the debug adapter (typically configured in `languages.toml`).

## PowerShell (PowerShellEditorServices)
A Debug Adapter Server for Powershell is not present by default, but can be added with the following configuration.

### Installation
Download the latest
[PowerShellEditorServices](https://github.com/PowerShell/PowerShellEditorServices/releases)
zip and extract it. In this example, I extracted it to C:\\.
You must then run `Get-ChildItem <PowerShellEditorServices-Path> -Recurse | Unblock-File` (in my case `Get-ChildItem C:\PowerShellEditorServices\ -Recurse | Unblock-File`) to remove the 'Mark of the Web' and allow PowerShellEditorServices to be executed.

After PowerShellEditorServices is present, you can add this config to your languages.toml (the correct `[[language]]` block is already present, if you followed my instructions to add the Language Server on the [Language-Server-Configurations](https://github.com/helix-editor/helix/wiki/Language-Server-Configurations#powershell) site on this Wiki):

```toml
[[language]]
name = 'powershell'
scope = 'source.ps1'
file-types = ['ps1', 'psm1']
roots = ['.git']
comment-token = '#'
indent = { tab-width = 4, unit = '\t' }
language-servers = [ 'powershell-editor-services' ]

[language.debugger]
name = 'powershell-editor-services'
transport = 'stdio'
command = 'pwsh'
args = ['-NoLogo', '-NoProfile', '-Command', 'C:\\PowerShellEditorServices\\PowerShellEditorServices\\Start-EditorServices.ps1 -SessionDetailsPath C:\\PowerShellEditorServices\\PowerShellEditorServices.sessions.dap.json -DebugServiceOnly -Stdio']

[[language.debugger.templates]]
name = 'Attach to existing Powershell Process'
request = 'attach'
completion = [ 'PID' ]
args = { processId = '{0}', runspaceId = '1' }

[[language.debugger.templates]]
name = 'Launch Script (Script runs in Background)'
request = 'launch'
completion = [ 'Full Script Path' ]
args = { script = '{0}', runspaceId = '1' }
```
### Things to know
First things first, I only tested this DAP Config on Windows with PowerShell 7. I don't know if it works under UNIX-Like Operating Systems (Linux and MAC) or if it works with PowerShell 6 and below.
#### Attaching
The template for attaching to an existing Powershell Process requires the PID of the Process. This PID is generally stored in the `$pid` System Variable. So you can just run `$pid` in a PowerShell Process, to get the PID for this Process.
After you gave Helix the PID, and it successfully attached to the PowerShell Process, you can use it to launch a Script you want to debug (don't forget to set your Breakpoints before you launch the Script).
#### Launching from within Helix
The template for launching a Script requires the FULL Path to the Script file (which is also why I didn't use Helix' 'File Picker' for this template). Additionally I haven't managed to launch a Script in the foreground, so if this Script is in any way interactive and/or if you need to actually see the console output, you need to use the 'attach' template instead of the 'launch' template.  
