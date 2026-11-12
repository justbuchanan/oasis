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
    cadquery.url = "github:vinszent/cq-flake?ref=main";
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
        # Pinned because newer nightlies' std fails to build for
        # riscv32imc-esp-espidf (no field `st_atime` on `libc::stat`).
        rustToolchain = pkgs.rust-bin.nightly."2026-02-11".default.override {
          extensions = [ "rust-src" ];
        };
      in
      {
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

        packages.default = self.packages.${system}.oasis-client;

        devShells.default =
          with pkgs;
          mkShell {
            buildInputs = [
              (kicad.override { withScripting = true; })
              cadquery.packages.${system}.cq-cli
              cadquery.packages.${system}.cq-editor
              # cq-warehouse has no tests and unittest now fails on "NO TESTS RAN"
              (cadquery.packages.${system}.cq-warehouse.overridePythonAttrs (_: {
                doCheck = false;
              }))
              gnumake
              nixfmt
              prettier
              openssl
              python312Packages.mpmath
              python312Packages.wxpython
              rustfmt
              rustToolchain
              shellcheck
              shfmt
              treefmt
              zlib
            ];

            # libclang for bindgen in ESP-IDF builds (esp-idf-sys)
            LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";

            shellHook = ''
              # Set IDF_TOOLS_PATH for ESP-IDF builds to use the local embuild directory
              export IDF_TOOLS_PATH="$PWD/code/esp32/.embuild/espressif"
              # Prioritize ESP-IDF venv site-packages over nix store packages
              # TODO: do better - this is pretty fragile
              if [ -d "$PWD/code/esp32/.embuild/espressif/python_env/idf5.4_py3.12_env/lib/python3.12/site-packages" ]; then
                export PYTHONPATH="$PWD/code/esp32/.embuild/espressif/python_env/idf5.4_py3.12_env/lib/python3.12/site-packages:$PYTHONPATH"
              fi

              # add pcbnew to python path
              export PYTHONPATH="$(dirname $(dirname $(readlink -f $(which kicad-cli))))/lib/python3.12/site-packages:$PYTHONPATH"

              # Add libraries to LD_LIBRARY_PATH for libclang (used by ESP-IDF bindgen)
              export LD_LIBRARY_PATH="${zlib}/lib:${stdenv.cc.cc.lib}/lib:$LD_LIBRARY_PATH"
            '';
          };
      }
    );
}
