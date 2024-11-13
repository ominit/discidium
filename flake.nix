{
  description = "a custom discord client written in rust";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };
      craneLib = crane.mkLib pkgs;
      nativeBuildInputs = with pkgs; [
        pkg-config
        gtk3
      ];

      buildInputs = with pkgs; [
        openssl
        wayland
        libxkbcommon
        libGL
      ];
    in {
      packages.default = craneLib.buildPackage {
        src = craneLib.cleanCargoSource ./.;

        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        RUST_BACKTRACE = 1;
      };
      devShell = pkgs.mkShell {
        inherit nativeBuildInputs buildInputs;
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        RUST_BACKTRACE = 1;
      };
    });
}
