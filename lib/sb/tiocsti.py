"""Writes a seccomp filter (BPF, for bubblewrap's --seccomp) that refuses the ioctls which push
input into a terminal (TIOCSTI) or reach the Linux console (TIOCLINUX). With it, a run keeps the
caller's terminal (job control, Ctrl-C) and still can't type commands into the shell that started
it, whatever the kernel's dev.tty.legacy_tiocsti. Usage: tiocsti.py OUT (at build time)."""
import sys
import termios

import seccomp

f = seccomp.SyscallFilter(defaction=seccomp.ALLOW)
for request in (termios.TIOCSTI, getattr(termios, "TIOCLINUX", 0x541C)):
    # the request is an int: compare its low 32 bits, as the kernel does
    f.add_rule(seccomp.ERRNO(1), "ioctl", seccomp.Arg(1, seccomp.MASKED_EQ, 0xFFFFFFFF, request))
with open(sys.argv[1], "wb") as out:
    f.export_bpf(out)
