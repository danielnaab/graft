# The Seatbelt renderer (macOS): a policy (policy.nix) as the launcher `sandbox` (launcher.nix, with
# seatbelt.sh), which runs a command under /usr/bin/sandbox-exec. Fixed here, when the launcher
# builds: the profile's first two parts, what a process needs from macOS (seatbelt.sb) and the
# closure's store paths, read-only (sb_store, from closureInfo: no other store path is readable).
# The launcher adds the run's part at each run: the repository and its pins, the host paths, the
# terminal, the proxy's port.
{ pkgs, lib, policy, nixpkgs, nixpkgsFrom, passthru }:

let
  inherit (policy) config;
  # the policy's packages, and what the launcher itself puts in a run: the shell, coreutils' env
  # (--default-signal), the CA bundle, terminfo
  closure = pkgs.closureInfo { rootPaths = config.packages ++ (with pkgs; [ bashInteractive coreutils cacert ncurses ]); };
  profile = pkgs.runCommand "sandbox.sb" { } ''
    source ${./seatbelt.sh}
    { cat ${./seatbelt.sb}; echo; echo "; the closure"; sb_store <${closure}/store-paths; } >$out
  '';
in
import ./launcher.nix { inherit pkgs lib policy nixpkgs nixpkgsFrom passthru; } {
  renderer = "Seatbelt";
  backend = ./seatbelt.sh;
  vars = ''
    profile=${profile}
    cacert=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt
  '';
  always = ''
    Always (Seatbelt): the closure read-only, and no other store path; the repository around the
    current directory (read-only with repository.readOnly = true); the git metadata that says where
    the repository is and what host git runs, read-only; HOME and TMPDIR new, empty folders; macOS's
    libraries and /private/etc read-only, and no other file; no Nix daemon; the caller's terminal,
    but no typing into it (TIOCSTI); the environment, only what the policy names.
  '';
}
