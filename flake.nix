{
  description = "Userland support for the ASUS Zenbook Duo UX8406";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      parts = pkgs.callPackage ./packaging/nix/package.nix { };
    in
    {
      packages.${system} = {
        asus-zenbook-duo-ux8406-second-screen = parts.second-screen;
        asus-zenbook-duo-ux8406-keyboard-bpf = parts.keyboard-bpf;
        asus-zenbook-duo-ux8406-tcc-guard = parts.tcc-guard;
        gnome-shell-extension-asus-zenbook-duo-ux8406-keys = parts.gnome-keys;
        gnome-shell-extension-builtin-screen-rotation = parts.gnome-rotation;
        default = parts.second-screen;
      };

      # NixOS: imports = [ asus-zenbook-duo-ux8406.nixosModules.default ];
      #
      # That is all the second-screen part needs. The keyboard part gets only
      # its hwdb entry and its udev rule, and the program is not loaded: nixpkgs has no udev-hid-bpf (October
      # 2026), whose udev rule and loader do that, and the loader looks for
      # the object in its own firmware directories, not in this store path.
      nixosModules.default =
        { ... }:
        {
          services.udev.packages = [
            self.packages.${system}.asus-zenbook-duo-ux8406-second-screen
            self.packages.${system}.asus-zenbook-duo-ux8406-keyboard-bpf
          ];
        };

      # NixOS: imports = [ asus-zenbook-duo-ux8406.nixosModules.tcc-guard ];
      #        services.asus-zenbook-duo-ux8406-tcc-guard.enable = true;
      # Runs the thermal guard. On other models it logs one line and ends.
      nixosModules.tcc-guard =
        { config, lib, ... }:
        {
          options.services.asus-zenbook-duo-ux8406-tcc-guard.enable =
            lib.mkEnableOption "the CPU thermal-offset guard for the Zenbook Duo UX8406";
          config = lib.mkIf config.services.asus-zenbook-duo-ux8406-tcc-guard.enable {
            systemd.packages = [ self.packages.${system}.asus-zenbook-duo-ux8406-tcc-guard ];
            systemd.services.asus-ux8406-tcc-guard.wantedBy = [ "multi-user.target" ];
          };
        };
    };
}
