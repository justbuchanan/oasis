# Usage:
# nix develop
#
# If on a non-nixos system and running something that uses opengl, you'll need
# to use the nixgl wrapper. For example:
# NixOS: `cq-editor`
# not NixOS: `nixGL cq-editor`
#
# TODO:
# * setup kicad plugins
# * cq-flake takes forever to build. figure out caching

{
  description = "Oasis dev environment";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0"; # Stable Nixpkgs
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    # My fork of cq-flake contains cq-cli, which isn't upstreamed yet
    cadquery.url = "github:justbuchanan/cq-flake?ref=main";
  };

  outputs =
    {
      self,
      nixpkgs,
      cadquery,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let

        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [ "rust-src" ];
        };

        # FHS environment for ESP32 development (allows ESP-IDF toolchain to run on NixOS)
        esp32-fhs = pkgs.buildFHSEnv {
          name = "esp32-fhs";
          targetPkgs = pkgs: with pkgs; [
            rustToolchain
            pkg-config
            cmake
            ninja
            python3
            python3Packages.pip
            python3Packages.virtualenv
            git
            wget
            gnumake
            flex
            bison
            gperf
            ncurses5
            zlib
            stdenv.cc
            libclang
            openssl
          ];
          runScript = "bash";
          profile = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [
              pkgs.stdenv.cc.cc.lib
              pkgs.zlib
              pkgs.ncurses5
            ]}
            export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
            # Unset PYTHONPATH to avoid conflicts with ESP-IDF venv
            unset PYTHONPATH
          '';
        };
      in
      {
        packages.esp32-fhs = esp32-fhs;
        packages.oasis-client = pkgs.rustPlatform.buildRustPackage {
          pname = "oasis";
          version = "0.1.0";
          src = ./code;
          sourceRoot = "code/client";
          cargoLock.lockFile = ./code/client/Cargo.lock;
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          buildInputs = with pkgs; [
            openssl
          ];
          postUnpack = ''
            cp -r code/terralib code/client/
          '';
          postInstall = ''
            # Rename the binary from 'client' to 'oasis'
            mv $out/bin/client $out/bin/oasis
          '';
        };

        packages.default = self.packages.${system}.oasis;

        devShells.default =
          with pkgs;
          mkShell {
            buildInputs = [
              (kicad.override { withScripting = true; })
              cadquery.packages.${pkgs.system}.cq-cli
              cadquery.packages.${pkgs.system}.cq-editor
              cadquery.packages.${pkgs.system}.cq-warehouse
              gnumake
              nixfmt-rfc-style
              nodePackages.prettier
              openssl
              python312Packages.mpmath
              python312Packages.wxpython
              rustfmt
              rustToolchain
              shellcheck
              shfmt
              treefmt
              zlib
              # ESP32 tooling - dependencies for ESP-IDF
              pkg-config
              cmake
              ninja
              python3
              git
              libclang
              ncurses5
              stdenv.cc.cc.lib
              espflash
            ];

            shellHook = ''
              # Add libraries to LD_LIBRARY_PATH for libclang (used by ESP-IDF bindgen)
              export LD_LIBRARY_PATH="${zlib}/lib:${stdenv.cc.cc.lib}/lib:${ncurses5}/lib:$LD_LIBRARY_PATH"

              # Set LIBCLANG_PATH for bindgen
              export LIBCLANG_PATH="${libclang.lib}/lib"

              # add pcbnew to python path
              export PYTHONPATH="$(dirname $(dirname $(readlink -f $(which kicad-cli))))/lib/python3.12/site-packages:$PYTHONPATH"

              # For ESP32 development on NixOS, use: nix run .#esp32-fhs
              # This provides an FHS environment where ESP-IDF toolchain binaries can run
              echo "Note: For ESP32 builds on NixOS, run 'nix run .#esp32-fhs' from the oasis root directory"
              echo "Then navigate to code/esp32 and run 'cargo build'"
            '';
          };
      }
    );
}
