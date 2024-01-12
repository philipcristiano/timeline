{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
        in
        with pkgs;
        {
          devShells.default = mkShell {
            buildInputs = [
                rust-bin.stable.latest.default
                darwin.apple_sdk.frameworks.Security # Should only be for darwin
                darwin.apple_sdk.frameworks.SystemConfiguration
                pkgs.postgresql_16
                pkgs.foreman
                pkgs.atlas
            ];
            shellHook = ''
              export PGDATA=$PWD/pgdata
              export DATABASE_URL="postgres://timeline@localhost/timeline?sslmode=disable"
            '';
          };
        }
      );
}
