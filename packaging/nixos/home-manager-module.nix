{ config, lib, pkgs, ... }:

let
  cfg = config.services.keynear;
  tomlFormat = pkgs.formats.toml { };

  defaultPackage =
    if pkgs ? keynear then
      pkgs.keynear.override { enableFuse = cfg.fuse; }
    else
      pkgs.callPackage ./package.nix { enableFuse = cfg.fuse; };

  defaultGnomeExtension =
    pkgs.keynear-gnome-extension or (pkgs.callPackage ./gnome-extension.nix { });

  forbiddenSecretKeys = [
    "irk"
    "reconnectSecret"
    "reconnect_secret"
    "privateKey"
    "private_key"
    "secret"
    "signingKey"
    "signing_key"
    "token"
    "password"
  ];

  collectKeys =
    attrs:
    lib.concatLists (
      lib.mapAttrsToList (
        name: value:
        [ name ] ++ lib.optionals (builtins.isAttrs value) (collectKeys value)
      ) attrs
    );

  renderedSettings = lib.filterAttrs (_: value: value != null) cfg.settings;
  secretKeysFound = lib.filter (k: builtins.elem k forbiddenSecretKeys) (collectKeys renderedSettings);
  configFilePath =
    if cfg.configDir != null then
      "${cfg.configDir}/config.toml"
    else
      null;
in
{
  options.services.keynear = {
    enable = lib.mkEnableOption "the Keynear continuity daemon and user integrations";

    package = lib.mkOption {
      type = lib.types.package;
      default = defaultPackage;
      defaultText = lib.literalExpression "pkgs.keynear";
      description = "The Keynear package providing keynear-linux and keynear-settings.";
    };

    fuse = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Whether to enable the on-demand FUSE mount (sets KEYNEAR_FUSE=1 and enables the fuse Cargo feature on the default package).";
    };

    logLevel = lib.mkOption {
      type = lib.types.enum [ "error" "warn" "info" "debug" "trace" ];
      default = "info";
      description = "KEYNEAR_LOG verbosity level for keynear-linux.";
    };

    configDir = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Optional custom KEYNEAR_CONFIG_DIR passed to keynear-linux.";
    };

    settings = lib.mkOption {
      default = { };
      description = "Non-secret DaemonConfig settings written to keynear/config.toml (pairwise secrets remain strictly in org.freedesktop.secrets).";
      type = lib.types.submodule {
        freeformType = tomlFormat.type;
        options = {
          visibilityMode = lib.mkOption {
            type = lib.types.nullOr (lib.types.enum [ "hidden" "pairedOnly" "everyoneNearby" ]);
            default = null;
            description = "Default discovery visibility mode.";
          };
          incomingDirectory = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            description = "Destination directory for incoming transfers.";
          };
          autoAcceptFromPaired = lib.mkOption {
            type = lib.types.nullOr lib.types.bool;
            default = null;
            description = "Whether incoming transfers from paired peers auto-accept by default.";
          };
          clipboardSyncEnabled = lib.mkOption {
            type = lib.types.nullOr lib.types.bool;
            default = null;
            description = "Enable background clipboard synchronization with paired devices.";
          };
          clipboardImagesEnabled = lib.mkOption {
            type = lib.types.nullOr lib.types.bool;
            default = null;
            description = "Include clipboard image payloads in clipboard sync.";
          };
          otpPassthroughEnabled = lib.mkOption {
            type = lib.types.nullOr lib.types.bool;
            default = null;
            description = "Accept verified one-time codes forwarded from paired Android peers.";
          };
          audioHandoffEnabled = lib.mkOption {
            type = lib.types.nullOr lib.types.bool;
            default = null;
            description = "Enable PipeWire and MPRIS2 smart Bluetooth headset handoff.";
          };
          ignoredMimeTypes = lib.mkOption {
            type = lib.types.nullOr (lib.types.listOf lib.types.str);
            default = null;
            description = "MIME types excluded from clipboard synchronization.";
          };
          notifyAutoAcceptComplete = lib.mkOption {
            type = lib.types.nullOr lib.types.bool;
            default = null;
            description = "Emit a desktop notification when an auto-accepted transfer finishes.";
          };
        };
      };
    };

    systemd.enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Manage the Keynear systemd --user service via Home Manager.";
    };

    integrations = {
      nautilus.enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Install the Send via Keynear script for GNOME Files (Nautilus).";
      };

      dolphin.enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Install the Send via Keynear KIO service menu for KDE Dolphin.";
      };

      yazi.enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Install the keynear.yazi plugin for Yazi.";
      };

      gnomeExtension = {
        enable = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Install the Keynear GNOME Shell clipboard bridge extension into home.packages.";
        };

        package = lib.mkOption {
          type = lib.types.package;
          default = defaultGnomeExtension;
          defaultText = lib.literalExpression "pkgs.keynear-gnome-extension";
          description = "The GNOME Shell clipboard bridge extension package.";
        };
      };
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = secretKeysFound == [ ];
        message = "services.keynear.settings must not contain secret keys (${lib.concatStringsSep ", " secretKeysFound}); secrets must stay in org.freedesktop.secrets.";
      }
      {
        assertion = !(renderedSettings ? incomingDirectory) || (lib.trim renderedSettings.incomingDirectory != "");
        message = "services.keynear.settings.incomingDirectory must not be empty.";
      }
    ];

    home.packages = [
      cfg.package
    ]
    ++ lib.optionals cfg.integrations.gnomeExtension.enable [
      cfg.integrations.gnomeExtension.package
    ];

    xdg.configFile = lib.mkMerge [
      (lib.mkIf (renderedSettings != { } && cfg.configDir == null) {
        "keynear/config.toml".source = tomlFormat.generate "keynear-config.toml" renderedSettings;
      })
      (lib.mkIf cfg.integrations.yazi.enable {
        "yazi/plugins/keynear.yazi/main.lua".source =
          "${cfg.package}/share/keynear/integrations/yazi/keynear.yazi/main.lua";
      })
    ];

    xdg.dataFile = lib.mkMerge [
      (lib.mkIf cfg.integrations.nautilus.enable {
        "nautilus/scripts/Send via Keynear" = {
          executable = true;
          text = ''
            #!/usr/bin/env bash
            set -euo pipefail
            paths=()
            if [ -n "''${NAUTILUS_SCRIPT_SELECTED_FILE_PATHS:-}" ]; then
              while IFS= read -r line; do
                if [ -n "$line" ]; then
                  paths+=("$line")
                fi
              done <<< "$NAUTILUS_SCRIPT_SELECTED_FILE_PATHS"
            elif [ "$#" -gt 0 ]; then
              paths=("$@")
            fi
            if [ "''${#paths[@]}" -eq 0 ]; then
              exit 0
            fi
            exec "${cfg.package}/bin/keynear-settings" --share -- "''${paths[@]}"
          '';
        };
      })
      (lib.mkIf cfg.integrations.dolphin.enable {
        "kio/servicemenus/keynear-send.desktop".source =
          "${cfg.package}/share/kio/servicemenus/keynear-send.desktop";
      })
    ];

    systemd.user.services.keynear = lib.mkIf cfg.systemd.enable {
      Unit = {
        Description = "Keynear background continuity daemon";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" "bluetooth.target" ];
        Wants = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart =
          if configFilePath != null && renderedSettings != { } then
            "${cfg.package}/bin/keynear-linux --config ${tomlFormat.generate "keynear-config.toml" renderedSettings}"
          else
            "${cfg.package}/bin/keynear-linux";
        Restart = "on-failure";
        RestartSec = 3;
        Environment = [
          "KEYNEAR_LOG=${cfg.logLevel}"
        ]
        ++ lib.optionals cfg.fuse [ "KEYNEAR_FUSE=1" ]
        ++ lib.optionals (cfg.configDir != null) [ "KEYNEAR_CONFIG_DIR=${cfg.configDir}" ];
        NoNewPrivileges = true;
        PrivateTmp = true;
        LockPersonality = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
      };
      Install = {
        WantedBy = [ "graphical-session.target" ];
      };
    };
  };
}

