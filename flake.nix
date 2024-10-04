{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { self, nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit system overlays; };
      nativeBuildInputs = [
        # Rust tools from toolchain file: includes cargo, compiler and
        # rust-anzalyzer.
        (pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
      ];
      buildInputs = [ ];
      forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed;
    in {
      devShells.${system}.default = pkgs.mkShell {
        inherit buildInputs nativeBuildInputs;
      };

      packages = forAllSystems (system: {
        default = pkgs.rustPlatform.buildRustPackage {
          inherit buildInputs nativeBuildInputs;
          pname = "git-repo-sync";
          version = "0.2.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
        };
      });

      defaultPackage = forAllSystems (system: self.packages.${system}.default);
    };
}
