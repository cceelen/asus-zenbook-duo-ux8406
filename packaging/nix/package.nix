# All parts as Nix packages. Used by ../../flake.nix; with plain Nix:
#   nix-build -E 'with import <nixpkgs> {}; (callPackage ./package.nix {}).second-screen'
#
# Each package is the Meson build of the tree with one part switched on
# (-Dscreen=true -Dguard=false ...), so that each carries its own files and
# licence. They are built from the tree itself, not from a source
# archive; the crates come from Cargo.lock through nixpkgs.
{
  lib,
  stdenv,
  cargo,
  rustc,
  rustPlatform,
  meson,
  ninja,
  pkg-config,
  llvmPackages,
  libbpf,
  linuxHeaders,
  removeReferencesTo,
  python3,
  glib,
  nodejs,
}:
let
  version = (lib.importTOML ../../Cargo.toml).workspace.package.version;
  src = lib.cleanSourceWith {
    src = ../..;
    filter =
      path: type:
      let
        name = baseNameOf path;
      in
      # cleanSourceFilter leaves out version control, editor backups, result
      # links and *.o. The directories below are build output: a tree that
      # was built in has them, and they must not reach the store.
      lib.cleanSourceFilter path type
      && name != "dist"
      && name != "target"
      && name != "build"
      && name != "__pycache__";
  };
  # The crates of the workspace, from Cargo.lock: dev-dependencies and the
  # optional fail crate of the guard's failpoints feature are all in there.
  # cargoSetupHook points cargo at them (the tree has no vendor/ here), so
  # the build resolves its crates in the sandbox without network.
  cargoDeps = rustPlatform.importCargoLock { lockFile = ../../Cargo.lock; };

  # One Meson build with the parts in `enable` switched on. `attrs` is what
  # differs between the packages (inputs, meta, ...); the Meson options and
  # the test run are the same for all.
  mkPart =
    {
      enable,
      attrs,
    }:
    let
      option = name: "-D${name}=${lib.boolToString (lib.elem name enable)}";
    in
    stdenv.mkDerivation (
      {
        inherit version src;

        strictDeps = true;

        mesonFlags = [
          (option "screen")
          (option "guard")
          (option "keyboard")
          (option "gnome")
          (option "rotation")
          # The guard goes to bin/, not sbin/ (nixpkgs has no sbin on PATH).
          "--sbindir=bin"
          "-Dudevdir=${placeholder "out"}/lib/udev"
          "-Dsystemdsystemunitdir=${placeholder "out"}/lib/systemd/system"
          "-Dfirmwaredir=${placeholder "out"}/lib/firmware"
          # Nix has no convention for licence files; they go below
          # share/licenses/<package>/.
          "-Dlicensedir=share/licenses"
        ] ++ (attrs.mesonFlags or [ ]);

        # `meson test` runs in the build; the Nix build user is not root.
        # The Rust tests build first, hence the time allowed.
        doCheck = true;
        mesonCheckFlags = [
          "--timeout-multiplier"
          "3"
        ];

        # The tests are run as the Nix build user, not root: name it in the log.
        preCheck = ''
          echo "tests run as: $(id)"
        '';

        # Meson's own list of what was installed, written into the licence
        # directory. It names build paths and is of no use in a package.
        postInstall = ''
          rm -f $out/share/licenses/depmf.json
        '';
      }
      // removeAttrs attrs [ "mesonFlags" ]
    );

  rustInputs = {
    inherit cargoDeps;
    nativeBuildInputs = [
      meson
      ninja
      pkg-config
      cargo
      rustc
      rustPlatform.cargoSetupHook
      # For the tests of the keyboard and extension parts.
      python3
      nodejs
    ];
    # The tree's Meson builds offline when it has vendor/ and otherwise asks
    # for it; here the crates are in cargoSetupHook's vendor directory.
    mesonFlags = [ "-Doffline=enabled" ];
  };
  meta = {
    homepage = "https://github.com/cceelen/asus-zenbook-duo-ux8406";
    platforms = [ "x86_64-linux" ];
  };
in
{
  # The helper and its udev rules. Add it to services.udev.packages.
  second-screen = mkPart {
    enable = [ "screen" ];
    attrs = rustInputs // {
      pname = "asus-zenbook-duo-ux8406-second-screen";
      meta = meta // {
        description = "Lower panel of the Zenbook Duo UX8406 off while the keyboard lies on it";
        license = lib.licenses.mit;
        mainProgram = "asus-ux8406-second-screen";
      };
    };
  };

  # The thermal guard and its systemd unit. Add it to systemd.packages and
  # enable it (the NixOS module of the flake does both).
  tcc-guard = mkPart {
    enable = [ "guard" ];
    attrs = rustInputs // {
      pname = "asus-zenbook-duo-ux8406-tcc-guard";
      meta = meta // {
        description = "CPU thermal-offset guard for the ASUS Zenbook Duo UX8406";
        license = lib.licenses.mit;
        mainProgram = "asus-ux8406-tcc-guard";
      };
    };
  };

  # The HID-BPF object and its hwdb entry, for udev-hid-bpf to load, and the
  # helper that keeps the keyboard's state, with its udev rule. Add it to
  # services.udev.packages.
  keyboard-bpf = mkPart {
    enable = [ "keyboard" ];
    attrs = rustInputs // {
      pname = "asus-zenbook-duo-ux8406-keyboard-bpf";
      nativeBuildInputs = rustInputs.nativeBuildInputs ++ [
        # The wrapped clang adds host flags that do not apply to the BPF
        # target; Meson finds this one as `clang`.
        llvmPackages.clang-unwrapped
        removeReferencesTo
      ];
      buildInputs = [
        libbpf
        linuxHeaders
      ];
      # The debug information of the object names the header files of clang,
      # libbpf and the kernel by their store paths. Nix would take these as
      # run-time dependencies (1.4 GiB, with LLVM) and an SBOM would list
      # them. The kernel does not read these names.
      postInstall = ''
        rm -f $out/share/licenses/depmf.json
        remove-references-to \
          -t ${llvmPackages.clang-unwrapped.lib} -t ${libbpf} -t ${linuxHeaders} \
          $out/lib/firmware/hid/bpf/*.bpf.o
      '';
      disallowedReferences = [
        llvmPackages.clang-unwrapped.lib
        libbpf
        linuxHeaders
      ];
      # clang reads these as extra include directories (the build files
      # have no option for them).
      env.C_INCLUDE_PATH = "${libbpf}/include:${linuxHeaders}/include";
      # The compiler wrapper's hardening flags do not apply to the BPF target.
      hardeningDisable = [ "all" ];
      dontStrip = true;
      dontFixup = true;
      meta = meta // {
        description = "HID-BPF program for the hotkeys of the Zenbook Duo UX8406 keyboard";
        license = with lib.licenses; [
          gpl2Only
          mit
        ];
      };
    };
  };

  # The GNOME Shell extension; list it in environment.systemPackages and
  # enable it per user.
  gnome-keys = mkPart {
    enable = [ "gnome" ];
    attrs = {
      pname = "gnome-shell-extension-asus-zenbook-duo-ux8406-keys";
      nativeBuildInputs = [
        meson
        ninja
        pkg-config
        glib
        nodejs
        python3
      ];
      passthru.extensionUuid = "asus-zenbook-duo-ux8406-keys@cceelen.github.io";
      meta = meta // {
        description = "GNOME Shell extension for the display keys of the Zenbook Duo UX8406";
        license = lib.licenses.mit;
      };
    };
  };

  # The GNOME Shell extension that turns the panels; it needs
  # hardware.sensor.iio.enable. List it in environment.systemPackages and
  # enable it per user.
  gnome-rotation = mkPart {
    enable = [ "rotation" ];
    attrs = {
      pname = "gnome-shell-extension-builtin-screen-rotation";
      nativeBuildInputs = [
        meson
        ninja
        pkg-config
        nodejs
        python3
      ];
      passthru.extensionUuid = "builtin-screen-rotation@cceelen.github.io";
      meta = meta // {
        description = "GNOME Shell extension that turns the built-in screens with the laptop";
        license = lib.licenses.mit;
      };
    };
  };
}
