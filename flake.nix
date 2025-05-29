{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    devShells.${system}.default = pkgs.mkShell {
      packages = with pkgs; [
        clang
        pkg-config
        libusb1
        glib
        python314
        usbutils
      ];
    };

    packages.${system}.default = pkgs.stdenv.mkDerivation {};
  };
}
