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
                self.overlays.local
              ];
            };
          in f { inherit pkgs; });
    in {
      overlays = {
        default = self.overlays.msg_q;
        msg_q = builtins.trace "loaded the overlay" (final: prev: {
          msg_q = builtins.trace "ran the overlay"
            self.packages.${prev.system}.default;
        });
        local = final: prev: {
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
      };

      packages = forEachSupportedSystem ({ pkgs }: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "msg_q";
          version = "0.1";
          cargoLock.lockFile = ./Cargo.lock;
          src = pkgs.lib.cleanSource ./.;
        };
      });

      nixosModules = {
        msg_q = { lib, config, pkgs, ... }:
          with lib;
          let cfg = config.services.msg_q;
          in {
            options.services.msg_q = {
              enable = mkEnableOption "msg_q";
              openFirewall = mkOption {
                description =
                  "Allow external access by allowing the port through the firewall";
                type = lib.types.bool;
                default = false;
                example = true;
              };
              port = mkOption {
                description = "Port to listen on";
                type = lib.types.int;
                default = 8080;
                example = 8080;
              };
            };
            config = lib.mkIf cfg.enable {

              networking.firewall.allowedTCPPorts =
                lib.mkIf cfg.openFirewall [ cfg.port ];
              systemd.services.msg_q = {
                wantedBy = [ "multi-user.target" ];
                serviceConfig.ExecStart = "${pkgs.msg_q}/bin/msg_q_server";
                environment = { SERVER_PORT = builtins.toString cfg.port; };
              };
            };
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
