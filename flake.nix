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
        gobject-introspection
        cargo-tauri
        rust-bin.stable.latest.default
      ];

      buildInputs = with pkgs; [
        at-spi2-atk
        atkmm
        cairo
        gdk-pixbuf
        glib
        gtk3
        harfbuzz
        librsvg
        libsoup_3
        pango
        webkitgtk_4_1
        openssl
      ];

      discidium = craneLib.buildPackage {
        name = "discidium";
        src = craneLib.cleanCargoSource ./.;
        inherit buildInputs nativeBuildInputs;

        postInstall = ''
          wrapProgram $out/bin/discidium \
            --prefix LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}"
        '';
      };
    in {
      packages.default = discidium;
      apps.default = flake-utils.lib.mkApp {
        drv = discidium;
      };

      devShell = pkgs.mkShell {
        inherit buildInputs nativeBuildInputs;
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        packages = with pkgs; [
          trunk
          (rust-bin.stable.latest.default.override {
            targets = ["wasm32-unknown-unknown" "x86_64-unknown-linux-gnu"];
          })
        ];
        # RUST_BACKTRACE = 1;
      };
    });
}
