#!/bin/bash

set -x

for f in repos/*; do
  git -C $f fetch
  git -C $f checkout origin/HEAD
done
