{
  description = "git-gist (gg) — run git across all child repositories";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rust = pkgs.rust-bin.stable.latest.default;
        git-gist = pkgs.rustPlatform.buildRustPackage {
          pname = "git-gist";
          version = "1.0.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ rust ];
          buildInputs = [ pkgs.git ];
          meta = with pkgs.lib; {
            description = "Run git commands across all child git repositories";
            homepage = "https://github.com/chtnnh/git-gist";
            license = licenses.mit;
            mainProgram = "gg";
            maintainers = [ ];
          };
        };
      in {
        packages.default = git-gist;
        packages.git-gist = git-gist;
        apps.default = {
          type = "app";
          program = "${git-gist}/bin/gg";
        };
        devShells.default = pkgs.mkShell {
          packages = [
            rust
            pkgs.git
            pkgs.cargo-llvm-cov
            pkgs.cargo-deb
          ];
        };
      });
}
