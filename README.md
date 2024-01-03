# ruSSH

A multi-host SSH client written in Rust.

### Usage

```bash
ruSSH "command1" "command2" "command3"
```

##### Optional Flags

`-c` - Pass a relative path to a `ruSSH.toml` value into the program.

```bash
ruSSH "command1" "command2" "command3" -c </path/to/ruSSH.toml>
```

### NixOS Flakes Installation

In `flake.nix` inputs add:

```nix
inputs = {
  ruSSH.url = "github:erictossell/russh";
}; 
```

In `flake.nix` modules add:

```nix
modules = [
  ({ pkgs, ruSSH, ... }: 
  {
    environment.systemPackages = with pkgs; [
      ruSSH.packages.${system}.default
    ];
  })
];
```

or

Imported as a `module.nix`:

```nix
{ pkgs, ruSSH, ... }: 
{
  environment.systemPackages = with pkgs; [
    ruSSH.packages.${system}.default
  ];
}
```

### Configuration
The first time running the application will ask if you would like to generate a `.config/ruSSH/ruSSH.toml` if one does not exist.

`ruSSH` will look for a `ruSSH.toml` in the `cwd` and if none exists it will default to the `.config/ruSSH` value. 

#### Example `ruSSH.toml`

```toml
servers = ["test.server.com"]

[ssh_options]
"test.server.com" = "-p 22"

[users]
"test.server.com" = "user"

```
