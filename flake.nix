{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};

    nativeBuildInputs = with pkgs; [
      rustc
      cargo
    ];
    buildInputs = with pkgs; [
      libusb1
    ];
  in {
    devShells.${system}.default = pkgs.mkShell {
      inherit nativeBuildInputs buildInputs;
      RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
      
      packages = with pkgs; [
        rust-analyzer
      ];
    };

    packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
      pname = "awoousb";
      version = "0.1.0";

      cargoHash = pkgs.lib.fakeHash;
    };
  };
}