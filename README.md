# RuSSH

A multi-host SSH client written in Rust.

### Usage

```bash
russh "command1" "command2" "command3" -c </path/to/russh.json>
```

`-c` - Pass a relative path to a `russh.json` value into the program.

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
The first time running the application will ask if you would like to generate a `.config/russh/russh.json` if one does not exist.

`russh` will look for a `russh.json` in the `cwd` and if none exists it will default to the `.config/russh` value. 

#### Example Configuration

```json
{
  "servers": ["192.168.2.195", "192.168.2.196", "192.168.2.197"],
  "ssh_options": {
    "192.168.2.195": "-p 2973",
    "192.168.2.196": "-p 2973",
    "192.168.2.197": "-p 2973"
  },
  "users": {
    "192.168.2.195": "eriim",
    "192.168.2.196": "eriim",
    "192.168.2.197": "eriim"
  }
}
```
