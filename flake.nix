{
  description = ''
    Simple NVIM-IDE launcher based on
    (Neovim-Session-Manager)https://github.com/Shatur/neovim-session-manager
  '';
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs = {
      nixpkgs.follows = "nixpkgs";
      flake-utils.follows = "flake-utils";
    };
    # TODO(Shvedov) Required unstable channel fot rust >= 1.70
    nixpkgs.url = github:NixOS/nixpkgs/nixos-unstable;
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        buildInputs = with pkgs; [
          gtk4
        ];
        nativeBuildInputs = with pkgs; [
          pkg-config
          wrapGAppsHook4
        ];
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      rec {
        packages = rec {
          ide-manager = pkgs.rustPlatform.buildRustPackage rec {
            pname = "ide-manager";
            version = "0.3.0";
            inherit nativeBuildInputs buildInputs;
            src = with builtins; path {
              filter = (path: type:
                let
                  bn = baseNameOf path;
                in
                bn != "flake.nix" && bn != "flake.lock"
              );
              path = self;
            };
            cargoLock.lockFile = "${self}/Cargo.lock";
          };
          default = ide-manager;
        };
        devShells.default = with pkgs; mkShellNoCC {
          buildInputs = [
            rust
            rust-analyzer-unwrapped
          ] ++ buildInputs ++ nativeBuildInputs;
          shellHook = ''
            export XDG_DATA_DIRS=$GSETTINGS_SCHEMAS_PATH
          '';
          RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/library";
        };
      }
    );
}
