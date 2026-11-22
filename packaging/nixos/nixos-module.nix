{ config, lib, pkgs, ... }:

let
  cfg = config.services.keynear;
  defaultPackage =
    if pkgs ? keynear then
      pkgs.keynear.override { enableFuse = cfg.fuse.enable; }
    else
      pkgs.callPackage ./package.nix { enableFuse = cfg.fuse.enable; };
  defaultGnomeExtension =
    pkgs.keynear-gnome-extension or (pkgs.callPackage ./gnome-extension.nix { });
in
{
  options.services.keynear = {
    enable = lib.mkEnableOption "the Keynear continuity daemon and desktop application";

    package = lib.mkOption {
      type = lib.types.package;
      default = defaultPackage;
      defaultText = lib.literalExpression "pkgs.keynear";
      description = "The Keynear package providing keynear, desktop entries, and icons.";
    };

    fuse = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable FUSE3 support, install fuse3, and set KEYNEAR_FUSE=1 for the user daemon.";
      };

      userAllowOther = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Set programs.fuse.userAllowOther when FUSE support is enabled.";
      };
    };

    bluetooth.enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable hardware.bluetooth by default so BLE discovery and audio handoff work.";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open mDNS discovery port (UDP 5353) and configured UDP ports in the host firewall.";
    };

    quicPort = lib.mkOption {
      type = lib.types.nullOr lib.types.port;
      default = null;
      description = "Fixed UDP port for the Keynear QUIC control channel; null keeps the daemon's dynamic-port default.";
    };

    extraAllowedUDPPorts = lib.mkOption {
      type = lib.types.listOf lib.types.port;
      default = [ ];
      description = "Additional UDP ports opened when services.keynear.openFirewall is true.";
    };

    allowedUDPPortRanges = lib.mkOption {
      type = lib.types.listOf (lib.types.attrsOf lib.types.port);
      default = [ ];
      example = [ { from = 40000; to = 40100; } ];
      description = "UDP port ranges opened when services.keynear.openFirewall is true.";
    };

    logLevel = lib.mkOption {
      type = lib.types.enum [ "error" "warn" "info" "debug" "trace" ];
      default = "info";
      description = "RUST_LOG / KEYNEAR_LOG verbosity level for the Keynear daemon.";
    };

    systemdUserService.enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Install and enable the hardened systemd --user service for graphical sessions.";
    };

    integrations.gnomeExtension = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Install the Keynear GNOME Shell clipboard bridge extension system-wide.";
      };

      package = lib.mkOption {
        type = lib.types.package;
        default = defaultGnomeExtension;
        defaultText = lib.literalExpression "pkgs.keynear-gnome-extension";
        description = "The GNOME Shell clipboard bridge extension package.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [
      cfg.package
    ]
    ++ lib.optionals cfg.fuse.enable [ pkgs.fuse3 ]
    ++ lib.optionals cfg.integrations.gnomeExtension.enable [
      cfg.integrations.gnomeExtension.package
    ];

    hardware.bluetooth.enable = lib.mkIf cfg.bluetooth.enable (lib.mkDefault true);

    programs.fuse.userAllowOther = lib.mkIf (cfg.fuse.enable && cfg.fuse.userAllowOther) true;

    networking.firewall = lib.mkIf cfg.openFirewall {
      allowedUDPPorts = [ 5353 ] ++ cfg.extraAllowedUDPPorts
        ++ lib.optionals (cfg.quicPort != null) [ cfg.quicPort ];
      allowedUDPPortRanges = cfg.allowedUDPPortRanges;
    };

    systemd.user.services.keynear = lib.mkIf cfg.systemdUserService.enable {
      description = "Keynear background continuity daemon";
      partOf = [ "graphical-session.target" ];
      after = [ "graphical-session.target" "bluetooth.target" ];
      wants = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/keynear daemon";
        Restart = "on-failure";
        RestartSec = 3;
        Environment = [
          "KEYNEAR_LOG=${cfg.logLevel}"
        ] ++ lib.optionals cfg.fuse.enable [
            "KEYNEAR_FUSE=1"
            "PATH=/run/wrappers/bin:/run/current-system/sw/bin"
          ]
          ++ lib.optionals (cfg.quicPort != null) [ "KEYNEAR_QUIC_PORT=${toString cfg.quicPort}" ];
        NoNewPrivileges = !cfg.fuse.enable;
        PrivateTmp = !cfg.fuse.enable;
        LockPersonality = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = !cfg.fuse.enable;
      };
    };
  };
}

