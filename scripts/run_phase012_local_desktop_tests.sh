#!/usr/bin/env sh
set -eu

test_files="$(find apps/desktop/tests -type f -name '*.ts' ! -name 'desktop_remote_product_smoke.ts' | sort)"
node --test $test_files
