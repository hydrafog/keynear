{ config, lib, ... }:

let
  cfg = config.services.keynear;
in
{
  options.services.keynear = {
    enable = lib.mkEnableOption "the Keynear continuity daemon";

    package = lib.mkOption {
      type = lib.types.package;
      description = "The Keynear package providing keynear-linux and keynear-settings.";
    };

    fuse = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Whether to enable the on-demand FUSE mount for paired remote roots.";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    systemd.user.services.keynear = {
      Unit = {
        Description = "Keynear continuity daemon";
        After = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${cfg.package}/bin/keynear-linux";
        Restart = "on-failure";
        Environment = lib.mkIf cfg.fuse [ "KEYNEAR_FUSE=1" ];
      };
      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
