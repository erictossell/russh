{
  description = "A very cool Rust application";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }@inputs:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.russh = pkgs.stdenv.mkDerivation {
          name = "russh";
          src = self;
          cargoSha256 = "0000000000000000000000000000000000000000000000000000";
          buildInputs = [ pkgs.rustc pkgs.cargo pkgs.gcc ];
          buildPhase = ''
            cargo build --release
          '';
          installPhase = ''
            install -D target/release/russh $out/bin/russh
          '';
        };

        defaultPackage = self.packages.${system}.russh;
      });
}

