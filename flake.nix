{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs {
        inherit system overlays;
      };
    in
    {

      devShells.default = with pkgs; mkShellNoCC {
        buildInputs = [ rust-bin.stable.latest.default
                        probe-run
                        rust-analyzer
                        pkg-config
                        gtk4
                      ];
        };
    });
}
