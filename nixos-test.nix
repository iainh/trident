# NixOS configuration for testing Trident on Linux
{ config, pkgs, ... }:

{
  # QEMU VM module will be imported in flake.nix

  # VM configuration
  virtualisation = {
    memorySize = 4096;   # 4GB RAM
    cores = 2;           # 2 CPU cores
    
    # QEMU-specific options
    qemu.options = [
      "-device virtio-vga"
    ];
    
    # Share the project directory with the VM
    sharedDirectories = {
      trident = {
        source = "$PWD";
        target = "/home/nixos/trident";
      };
    };
  };

  # Enable X11 and desktop environment for hotkey testing
  services.xserver.enable = true;
  services.displayManager.gdm.enable = true;
  services.desktopManager.gnome.enable = true;
  
  # Enable input devices
  services.libinput.enable = true;

  # Enable audio (not needed for Trident but useful for complete desktop)
  # Use PipeWire instead of PulseAudio (more modern)
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
  };

  # Install necessary packages for testing
  environment.systemPackages = with pkgs; [
    # Development tools
    rustc
    cargo
    pkg-config
    openssl

    # X11 tools for window management testing  
    wmctrl
    xdotool
    xorg.xev  # For testing keycode detection

    # Terminal emulators for testing
    gnome-terminal
    alacritty
    kitty
    xterm
    
    # Desktop file utilities
    desktop-file-utils
    
    # SSH for testing SSH connections
    openssh
    
    # Debugging tools
    strace
    gdb
    
    # Text editors
    vim
    nano
    
    # Network tools for SSH testing
    netcat
  ];

  # Enable SSH server for testing SSH connections
  services.openssh = {
    enable = true;
    settings = {
      PasswordAuthentication = true;
      PermitRootLogin = "yes";
    };
  };

  # Create a test user
  users.users.testuser = {
    isNormalUser = true;
    password = "test";
    extraGroups = [ "wheel" ];
    shell = pkgs.bash;
  };

  # Enable sudo without password for convenience
  security.sudo.wheelNeedsPassword = false;

  # Configure the default user
  users.users.nixos = {
    isNormalUser = true;
    password = "nixos";
    extraGroups = [ "wheel" ];
  };

  # Set timezone
  time.timeZone = "UTC";

  # Enable experimental features for testing
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # System version
  system.stateVersion = "23.11";
}