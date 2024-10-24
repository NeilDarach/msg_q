{
  description = "A Devshell for rust development";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = { nixpkgs.follows = "nixpkgs"; };
    };
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils, nixNvim, rust-overlay, ... }@inputs:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        rustToolchain = let rust = pkgs.rust-bin;
        in if builtins.pathExists ./rust-toolchain.toml then
          rust.fromRustupToolchainFile ./rust-toolchain.toml
        else if builtins.pathExists ./rust-toolchain then
          rust.fromRustToolchainFile ./rust-toolchain
        else
          rust.stable.latest.default.override {
            extensions = [ "rust-src" "rustfmt" ];
          };

      in {
        packages.default =
          let manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
          in pkgs.rustPlatform.buildRustPackage {
            pname = manifest.name;
            version = manifest.version;
            cargoLock.lockFile = ./Cargo.lock;
            src = pkgs.lib.cleanSource ./.;
          };

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

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [ pkgs.rust-analyzer rustToolchain just bacon ];
          env = {
            RUST_SRC_PATH =
              "${rustToolchain}/lib/rustlib/src/rust/library";
          };
        };
      });
}
