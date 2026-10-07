# Security

The guard and the second-screen helper run as root. Report a vulnerability in
private:

1. Open the
   [Security tab](https://github.com/cceelen/asus-zenbook-duo-ux8406/security)
   of the repository.
2. Select "Report a vulnerability".

Do not open a public issue for a vulnerability.

Only the latest release gets fixes.

The service that reads the kernel log is a workaround until the BIOS or the
kernel handles the thermal event. Its parsers have tests for hostile input and
fuzz targets (`fuzz/`): the records of the kernel log, the numbers from sysfs,
the configuration file, and the notes of the offsets found. CI gives each target
its saved inputs on every change, and fuzzes each target for five minutes each
night. An input that makes a target fail gets a private draft security advisory.
