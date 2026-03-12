# Lift

A [nxloader](https://github.com/XtremeTHN/NXLoader) rust port. This port should solve some issues with usb transfers and memory errors.

## Installation
~~Currently only flatpak is supported, cuz this program uses desktop portals to access the switch usb.<br>~~

### Flatpak
Download the flatpak file in the releases or build it yourself with gnome, and then execute this:
```
flatpak install --user com.github.XtremeTHN.Lift.flatpak
```
Done

### Meson
#### Dependencies
- libusb
- gtk4
- libadwaita
- libgudev

```
meson setup build
meson install -C build
```

### NixOs
Add this repository to your flake inputs
```nix
# flake.nix
inputs.lift = {
  url = "github:XtremeTHN/Lift;
  inputs.nixpkgs.follows = "nixpkgs";
};
```
Then add lift to the `home.packages` or `environment.systemPackages`.
```nix
# home.nix
home.packages = [
  inputs.lift.packages."x86_64-linux".default
];
```

## TODO
- [ ] Add support for wireless protocol

## Preview
<img width="650" height="550" alt="switch not connected page" src="https://github.com/user-attachments/assets/e661fccd-2c34-4366-b110-3b9f9599ba68" />
<img width="650" height="550" alt="rom selection page" src="https://github.com/user-attachments/assets/33934b4d-681e-4363-8472-57f606e40b58" />
<img width="650" height="550" alt="rom upload" src="https://github.com/user-attachments/assets/2e8afd49-f728-42a6-9020-86fd8db52f17" />
