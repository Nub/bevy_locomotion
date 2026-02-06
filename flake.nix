{
  description = "First-person character controller with Bevy 0.18 and Avian3D";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Runtime libraries needed by Bevy
        runtimeDeps = with pkgs; [
          vulkan-loader
          alsa-lib
          udev
        ] ++ (with pkgs.xorg; [
          libX11
          libXcursor
          libXi
          libXrandr
        ]) ++ [
          pkgs.libxkbcommon
          pkgs.wayland
        ];

        # Build-time libraries for compiling
        buildLibs = with pkgs; [
          wayland
          wayland.dev
        ];

        # Build-time dependencies
        buildDeps = with pkgs; [
          pkg-config
          clang
          mold
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = buildDeps ++ buildLibs ++ runtimeDeps ++ [ rustToolchain ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeDeps;

          PKG_CONFIG_PATH = "${pkgs.wayland.dev}/lib/pkgconfig";

          RUST_BACKTRACE = 1;

          shellHook = ''
            echo "Bevy 0.18 + Avian3D development environment"
            echo "Run: cargo run --features dev"
          '';
        };
      }
    );
}
