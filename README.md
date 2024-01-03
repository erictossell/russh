# RuSSH

A multi-host SSH client written in Rust.

### Usage

```bash
russh "command1" "command2" "command3"
```

##### Optional Flags

`-c` - Pass a relative path to a `russh.toml` value into the program.

```bash
russh "command1" "command2" "command3" -c </path/to/russh.toml>
```

### NixOS Flakes Installation

In `flake.nix` inputs add:

```nix
inputs = {
  russh.url = "github:erictossell/russh";
}; 
```

In `flake.nix` modules add:

```nix
modules = [
  ({ pkgs, russh, ... }: 
  {
    environment.systemPackages = with pkgs; [
      russh.packages.${system}.default
    ];
  })
];
```

or

Imported as a `module.nix`:

```nix
{ pkgs, russh, ... }: 
{
  environment.systemPackages = with pkgs; [
    russh.packages.${system}.default
  ];
}
```

### Configuration
The first time running the application will ask if you would like to generate a `.config/russh/russh.toml` if one does not exist.

`russh` will look for a `russh.toml` in the `cwd` and if none exists it will default to the `.config/russh` value. 

#### Example Configuration

```toml
servers = ["test.server.com"]

[ssh_options]
"test.server.com" = "-p 22"

[users]
"test.server.com" = "user"

```
