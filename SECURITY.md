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
fuzz targets (`tcc-guard/fuzz`, run with `cargo +nightly fuzz run kmsg`).
