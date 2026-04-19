# Lift

A [nxloader](https://github.com/XtremeTHN/NXLoader) rust port. This port should solve some issues with usb transfers and memory errors.

## Installation
~~Currently only flatpak is supported, cuz this program uses desktop portals to access the switch usb.<br>~~

### Flatpak
Download the flatpak file in the releases or build it yourself with gnome builder, and then execute this:
```
flatpak install --user com.github.XtremeTHN.Lift.flatpak
```
Done

### General
#### Dependencies
- `libusb`
- `gtk4`
- `libadwaita`
- `libgudev`

Use meson:

```
meson setup build
meson install -C build
```

### NixOs
Add this repository to your flake inputs
```nix
# flake.nix
inputs.lift = {
  url = "github:XtremeTHN/Lift";
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
- [x] Add support for wireless protocol

## Preview
<img width="430" height="602" alt="Home page (usb)" src="https://github.com/user-attachments/assets/ae574f53-2a72-4485-8e94-fb280dc772b8" />
<img width="430" height="602" alt="Home page (wireless)" src="https://github.com/user-attachments/assets/e2c4d59d-43a4-4826-822c-ec721e7123d0" />
<img width="628" height="621" alt="Usb device permission dialog (only in flatpak) (usb portal)" src="https://github.com/user-attachments/assets/1c389b23-cbe4-4224-b4c3-213ca8cb8c5d" />
<img width="469" height="607" alt="Rom transfer" src="https://github.com/user-attachments/assets/83bbbe5b-da26-48e8-8a3d-42886fd81724" />

