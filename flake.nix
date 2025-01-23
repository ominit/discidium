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
      rustToolchainFor = p:
        p.rust-bin.stable.latest.default.override {
          targets = ["wasm32-unknown-unknown"];
        };

      inherit (pkgs) lib;
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchainFor;

      unfilteredRoot = ./.;
      src = lib.fileset.toSource {
        root = unfilteredRoot;
        fileset = lib.fileset.unions [
          (craneLib.fileset.commonCargoSources unfilteredRoot)
          ./Cargo.toml
          ./tailwind.config.js
          (lib.fileset.maybeMissing ./src-tauri)
          (lib.fileset.maybeMissing ./public)
          (lib.fileset.fileFilter (file: lib.any file.hasExt ["html" "css"]) unfilteredRoot)
        ];
      };

      nativeBuildInputs = with pkgs; [
        pkg-config
        gobject-introspection
        cargo-tauri.hook
        (rust-bin.stable.latest.default.override {
          targets = ["wasm32-unknown-unknown" "x86_64-unknown-linux-gnu"];
        })
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
        librsvg
      ];

      discidium-trunk = craneLib.buildPackage {
        inherit src;
        strictDeps = true;
        CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
        cargoArtifacts = craneLib.buildDepsOnly {
          inherit src;
          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
          doCheck = false;
        };
        doCheck = false;
        pnameSuffix = "-trunk";

        preConfigure = ''
          echo configuring trunk tools
          TRUNK_TOOLS_SASS=$(sass --version | head -n1)
          TRUNK_TOOLS_WASM_BINDGEN=$(wasm-bindgen --version | cut -d' ' -f2)
          TRUNK_TOOLS_WASM_OPT="version_$(wasm-opt --version | cut -d' ' -f3)"
          export TRUNK_TOOLS_SASS
          export TRUNK_TOOLS_WASM_BINDGEN
          export TRUNK_TOOLS_WASM_OPT

          echo "TRUNK_TOOLS_SASS=''${TRUNK_TOOLS_SASS}"
          echo "TRUNK_TOOLS_WASM_BINDGEN=''${TRUNK_TOOLS_WASM_BINDGEN}"
          echo "TRUNK_TOOLS_WASM_OPT=''${TRUNK_TOOLS_WASM_OPT}"
        '';

        buildPhaseCargoCommand = ''
          trunk build --release=true --offline
        '';

        installPhaseCommand = ''
          cp -r ./dist $out
        '';

        doNotPostBuildInstallCargoBinaries = true;
        doInstallCargoArtifacs = false;

        wasm-bindgen-cli = pkgs.wasm-bindgen-cli.override {
          version = "0.2.100";
          hash = "sha256-3RJzK7mkYFrs7C/WkhW9Rr4LdP5ofb2FdYGz1P7Uxog=";
          cargoHash = "sha256-tD0OY2PounRqsRiFh8Js5nyknQ809ZcHMvCOLrvYHRE=";
          # When updating to a new version comment out the above two lines and
          # uncomment the bottom two lines. Then try to do a build, which will fail
          # but will print out the correct value for `hash`. Replace the value and then
          # repeat the process but this time the printed value will be for `cargoHash`
          # hash = lib.fakeHash;
          # cargoHash = lib.fakeHash;
        };

        nativeBuildInputs = with pkgs; [binaryen dart-sass trunk wasm-bindgen-cli tailwindcss];
      };

      discidium = craneLib.buildPackage {
        pname = "discidium";
        inherit buildInputs nativeBuildInputs src;
        cargoArtifacts = craneLib.buildDepsOnly {
          postUnpack = ''
            cd $sourceRoot/src-tauri
            sourceRoot="."
          '';
          inherit buildInputs nativeBuildInputs src;
          postInstall = ''
            cd ../
          '';
          doCheck = false;
        };
        doCheck = false;
        preConfigure = ''
          cp -r ${discidium-trunk} ./dist
        '';
        buildPhaseCargoCommand = ''
          cargo tauri build
        '';
        installPhaseCommand = ''
          cp -r ./target/release/bundle/deb/*/data/usr $out
          mv $out/bin/discidium-tauri $out/bin/discidium
        '';
        doNotPostBuildInstallCargoBinaries = true;
      };
    in {
      packages.default = discidium;
      apps.default = flake-utils.lib.mkApp {
        drv = discidium;
      };

      devShell = pkgs.mkShell {
        inherit buildInputs nativeBuildInputs;
        # LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        packages = with pkgs; [
          trunk
        ];
        # RUST_BACKTRACE = 1;
      };
    });
}
