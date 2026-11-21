#!/bin/sh
# Build the USTAR initrd from initrd/ directory (GNU tar, no compression).
set -eu
cd "$(dirname "$0")/.."
tar --format=ustar -cf initrd.img -C initrd welcome.txt version.txt
