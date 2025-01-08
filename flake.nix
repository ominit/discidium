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
      lib = nixpkgs.lib;
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };
      craneLib = crane.mkLib pkgs;

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = [
          "rust-src"
          "rust-analyzer"
          "clippy"
        ];
      };
      rustBuildInputs =
        [
          pkgs.openssl
          pkgs.libiconv
          pkgs.pkg-config
        ]
        ++ lib.optionals pkgs.stdenv.isLinux [
          pkgs.glib
          pkgs.gtk3
          pkgs.libsoup_3
          pkgs.webkitgtk_4_1
          pkgs.xdotool
        ]
        ++ lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [
          IOKit
          Carbon
          WebKit
          Security
          Cocoa
        ]);

      discidium = craneLib.buildPackage {
        name = "discidium";
        src = lib.cleanSourceWith {
          src = self;
          filter = path: type:
            (lib.hasSuffix "\.html" path)
            || (lib.hasInfix "/assets/" path)
            || (craneLib.filterCargoSources path type);
        };
        buildInputs = rustBuildInputs;
        nativeBuildInputs = [
          rustToolchain
        ];

        postInstall = ''
          wrapProgram $out/bin/discidium \
            --prefix LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath rustBuildInputs}"
        '';
      };
    in {
      packages.default = discidium;
      apps.default = flake-utils.lib.mkApp {
        drv = discidium;
      };

      devShell = pkgs.mkShell {
        buildInputs = rustBuildInputs;
        nativeBuildInputs = [
          rustToolchain
        ];
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath rustBuildInputs;
        packages = with pkgs; [
          (rust-bin.stable.latest.default.override {
            targets = ["wasm32-unknown-unknown" "x86_64-unknown-linux-gnu"];
          })
        ];
        shellHook = ''
          cargo install dioxus-cli
        '';
        # RUST_BACKTRACE = 1;
      };
    });
}
