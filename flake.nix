{
  description = "A Devshell for rust development";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixnvim = {
      url = "github:NeilDarach/nixNvim";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
      };
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = { nixpkgs.follows = "nixpkgs"; };
    };
  };

  outputs = { self, nixpkgs, nixnvim, rust-overlay, ... }@inputs:
    let
      supportedSystems =
        [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forEachSupportedSystem = f:
        nixpkgs.lib.genAttrs supportedSystems (system:
          let
            inherit (nixnvim) utils;
            nvim = nixnvim.packages.${system}.default.override (prev: {
              categoryDefinitions = utils.mergeCatDefs prev.categoryDefinitions
                ({ pkgs, settings, categories, name, ... }@packageDef: {
                  environmentVariables = {
                    general = { FROMDEVSHELL = "yes"; };
                  };
                });
              packageDefinitions = prev.packageDefinitions // {
                nvim = utils.mergeCatDefs prev.packageDefinitions.nvim
                  ({ pkgs, ... }: { categories = { rust = true; }; });
              };
            });

            pkgs = import nixpkgs {
              inherit system;
              overlays = [
                (_: _: { inherit nvim; })
                rust-overlay.overlays.default
                self.overlays.default
              ];
            };
          in f { inherit pkgs; });
    in {
      overlays.default = final: prev: {
        rustToolchain = let rust = prev.rust-bin;
        in if builtins.pathExists ./rust-toolchain.toml then
          rust.fromRustupToolchainFile ./rust-toolchain.toml
        else if builtins.pathExists ./rust-toolchain then
          rust.fromRustupToolchainFile ./rust-toolchain
        else
          rust.stable.latest.default.override {
            extensions = [ "rust-src" "rustfmt" ];
          };
      };
      devShells = forEachSupportedSystem ({ pkgs }: {
        default = pkgs.mkShell {
          packages = with pkgs; [ nvim rustToolchain just bacon ];
          env = {
            RUST_SRC_PATH =
              "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
          };
        };
      });
    };
}
