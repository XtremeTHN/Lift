from subprocess import check_output, run, PIPE
import sys

DEPENDENCIES = [
    "glib-2.0",
    "gio-2.0",
    "libusb-1.0",
]

COMPILER = "gcc"
OUT_FILE = "usb"

def dump_output(proc):
    if proc.stdout:
        print(proc.stdout.decode())
    if proc.stderr:
        print(proc.stderr.decode())

def build():
    out = check_output(["pkg-config", "--cflags", "--libs"] + DEPENDENCIES, text=True)
    flags = [x.strip() for x in out.split(" ")]

    proc = run([COMPILER, "src/main.c"] + flags + ["-o", "usb"], stderr=PIPE, stdout=PIPE)
    if proc.returncode != 0:
        print("Build failed")
        dump_output(proc)
        sys.exit(1)

def _run():
    proc = run(["sudo", "G_MESSAGES_DEBUG=all", "./" + OUT_FILE])
    if proc.returncode != 0:
        print("Exit code:", proc.returncode)
        dump_output(proc)
        sys.exit(1)

if __name__ == "__main__":
    build()
    _run()