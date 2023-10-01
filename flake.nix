{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
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
      in
      rec {
        packages = rec {
          ide-manager = pkgs.rustPlatform.buildRustPackage rec {
            pname = "ide-manager";
            version = "0.2";
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
            rust-bin.stable.latest.default
            probe-run
            rust-analyzer
          ] ++ buildInputs ++ nativeBuildInputs;
          shellHook = ''
            export XDG_DATA_DIRS=$GSETTINGS_SCHEMAS_PATH
          '';
        };
      }
    );
}
